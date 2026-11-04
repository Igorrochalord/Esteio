//! Financeiro: contas, títulos (parcelas, baixa, estorno), ponte com vendas, transferências,
//! extrato OFX, fluxo de caixa e DRE.

mod common;

use std::str::FromStr;

use common::*;
use nucleo_core::modules::customers::models::CustomerInput;
use nucleo_core::modules::customers::service as customers;
use nucleo_core::modules::finance::models::*;
use nucleo_core::modules::finance::reports::{self, CashFlowQuery, DreQuery};
use nucleo_core::modules::finance::{ofx, service as fin};
use nucleo_core::modules::products::models::ProductInput;
use nucleo_core::modules::products::service as products;
use nucleo_core::modules::sales::models::{SaleInput, SaleItemInput};
use nucleo_core::modules::sales::service as sales;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, CoreError, Ctx};
use rust_decimal::Decimal;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

async fn ctx(core: &Core) -> Ctx {
    core.require(Module::Finance, Action::Manage).await.unwrap()
}

fn account(name: &str, initial: &str) -> AccountInput {
    AccountInput {
        name: name.into(),
        kind: "corrente".into(),
        bank_name: Some("Banco X".into()),
        agency: None,
        account_number: None,
        initial_balance: Some(d(initial)),
        opening_date: Some("2026-01-01".into()),
        is_default: None,
    }
}

async fn category(ctx: &Ctx, name: &str) -> String {
    fin::categories(ctx)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap()
        .id
}

fn entry(kind: &str, category_id: &str, amount: &str, due: &str) -> EntryInput {
    EntryInput {
        kind: kind.into(),
        description: "Título".into(),
        category_id: category_id.into(),
        customer_id: None,
        counterparty: Some("Fornecedor".into()),
        amount: d(amount),
        issue_date: Some(due.into()),
        due_date: due.into(),
        installments: None,
        repeat_months: None,
        document: None,
        notes: None,
    }
}

async fn balance(ctx: &Ctx, id: &str) -> Decimal {
    fin::accounts(ctx)
        .await
        .unwrap()
        .into_iter()
        .find(|a| a.id == id)
        .unwrap()
        .balance
}

#[tokio::test]
async fn accounts_default_and_balances() {
    let (_dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let a = fin::create_account(&c, account("Itaú", "1000"))
        .await
        .unwrap();
    assert!(a.is_default, "a primeira conta é a padrão");
    assert_eq!(a.balance, d("1000"));
    let mut second = account("Caixa loja", "50");
    second.is_default = Some(true);
    let b = fin::create_account(&c, second).await.unwrap();
    let list = fin::accounts(&c).await.unwrap();
    assert!(list.iter().find(|x| x.id == b.id).unwrap().is_default);
    assert!(!list.iter().find(|x| x.id == a.id).unwrap().is_default);
    let off = AccountPatch {
        active: Some(false),
        ..Default::default()
    };
    assert!(matches!(
        fin::update_account(&c, &b.id, off).await,
        Err(CoreError::BusinessRule(_))
    ));
    assert!(matches!(
        fin::create_account(&c, account("itaú", "0")).await,
        Err(CoreError::Conflict(_))
    ));
}

#[tokio::test]
async fn installments_payment_and_reversal() {
    let (_dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let acc = fin::create_account(&c, account("Banco", "0"))
        .await
        .unwrap();
    let aluguel = category(&c, "Aluguel").await;
    let vendas = category(&c, "Vendas").await;

    // categoria de receita num título a pagar é recusada
    assert!(matches!(
        fin::create_entry(&c, entry("pagar", &vendas, "10", "2026-10-10")).await,
        Err(CoreError::Validation(_))
    ));

    let mut parcelado = entry("pagar", &aluguel, "100", "2026-01-31");
    parcelado.installments = Some(3);
    let parts = fin::create_entry(&c, parcelado).await.unwrap();
    let amounts: Vec<Decimal> = parts.iter().map(|e| e.amount).collect();
    assert_eq!(amounts, [d("33.33"), d("33.33"), d("33.34")]);
    let dues: Vec<&str> = parts.iter().map(|e| e.due_date.as_str()).collect();
    assert_eq!(
        dues,
        ["2026-01-31", "2026-02-28", "2026-03-31"],
        "fim de mês respeitado"
    );
    assert_eq!(parts[0].description, "Título (1/3)");

    let mut recorrente = entry("pagar", &aluguel, "50", "2026-05-05");
    recorrente.repeat_months = Some(3);
    assert!(fin::create_entry(&c, recorrente)
        .await
        .unwrap()
        .iter()
        .all(|e| e.amount == d("50")));

    // baixa com juros: paga 35,00 de um título de 33,33
    let pay = PayInput {
        date: "2026-02-01".into(),
        amount: Some(d("35")),
        bank_account_id: Some(acc.id.clone()),
        payment_method: Some("pix".into()),
    };
    let paid = fin::pay_entry(&c, &parts[0].id, pay).await.unwrap();
    assert_eq!(
        (paid.status.as_str(), paid.paid_amount),
        ("pago", Some(d("35")))
    );
    assert_eq!(balance(&c, &acc.id).await, d("-35"));
    let again = PayInput {
        date: "2026-02-01".into(),
        amount: None,
        bank_account_id: None,
        payment_method: None,
    };
    assert!(matches!(
        fin::pay_entry(&c, &parts[0].id, again).await,
        Err(CoreError::BusinessRule(_))
    ));
    assert!(matches!(
        fin::cancel_entry(&c, &parts[0].id).await,
        Err(CoreError::BusinessRule(_))
    ));

    // o lançamento da baixa não pode ser apagado direto; estornar devolve o saldo
    let tx = fin::transactions(&c, TxQuery::default()).await.unwrap();
    assert!(matches!(
        fin::delete_transaction(&c, &tx[0].id).await,
        Err(CoreError::BusinessRule(_))
    ));
    let back = fin::unpay_entry(&c, &parts[0].id).await.unwrap();
    assert_eq!(back.status, "aberto");
    assert_eq!(balance(&c, &acc.id).await, d("0"));
    assert_eq!(
        fin::cancel_entry(&c, &parts[1].id).await.unwrap().status,
        "cancelado"
    );

    let open = fin::entries(
        &c,
        EntryQuery {
            kind: Some("pagar".into()),
            status: Some("aberto".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(open.len(), 5);
}

#[tokio::test]
async fn sales_feed_finance() {
    let (_dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let p = products::create(
        &c,
        ProductInput {
            code: String::new(),
            barcode: None,
            name: "Bolo".into(),
            category_id: None,
            unit: None,
            price: d("30"),
            cost: Some(d("12")),
            track_stock: Some(false),
            min_stock: None,
            initial_stock: None,
        },
    )
    .await
    .unwrap();
    let maria = customers::create(
        &c,
        CustomerInput {
            name: "Maria".into(),
            document: None,
            phone: None,
            email: None,
            city: None,
            state: None,
            notes: None,
        },
    )
    .await
    .unwrap();
    let sale = |pay: &str, customer: Option<String>| SaleInput {
        customer_id: customer,
        seller_id: None,
        payment_method: pay.into(),
        discount: None,
        notes: None,
        sold_at: Some("2026-10-02T01:30:00Z".into()), // 01/10 22:30 em Brasília
        tz_offset_minutes: Some(-180),
        items: vec![SaleItemInput {
            product_id: p.id.clone(),
            qty: d("2"),
            unit_price: None,
            discount: None,
        }],
    };

    // à vista: cria "Caixa" sozinho e lança na data local
    let cash = sales::create(&c, sale("pix", None)).await.unwrap();
    let accounts = fin::accounts(&c).await.unwrap();
    assert_eq!(
        (
            accounts.len(),
            accounts[0].name.as_str(),
            accounts[0].balance
        ),
        (1, "Caixa", d("60"))
    );
    let tx = fin::transactions(&c, TxQuery::default()).await.unwrap();
    assert_eq!(
        (tx[0].date.as_str(), tx[0].origin.as_str()),
        ("2026-10-01", "venda")
    );

    // fiado: conta a receber de 30 dias com o cliente
    let fiado = sales::create(&c, sale("fiado", Some(maria.id.clone())))
        .await
        .unwrap();
    let rec = fin::entries(
        &c,
        EntryQuery {
            kind: Some("receber".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rec.len(), 1);
    assert_eq!(
        (
            rec[0].due_date.as_str(),
            rec[0].customer_name.as_deref(),
            rec[0].amount
        ),
        ("2026-10-31", Some("Maria"), d("60"))
    );
    assert_eq!(rec[0].sale_number, Some(fiado.number));
    assert!(matches!(
        fin::cancel_entry(&c, &rec[0].id).await,
        Err(CoreError::BusinessRule(_))
    ));

    // cancelar as vendas desfaz o financeiro
    sales::cancel(&c, &cash.id, "teste").await.unwrap();
    sales::cancel(&c, &fiado.id, "teste").await.unwrap();
    assert_eq!(fin::accounts(&c).await.unwrap()[0].balance, d("0"));
    let rec = fin::entries(
        &c,
        EntryQuery {
            kind: Some("receber".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rec[0].status, "cancelado");
}

#[tokio::test]
async fn transfers_and_manual_entries() {
    let (_dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let a = fin::create_account(&c, account("A", "100")).await.unwrap();
    let b = fin::create_account(&c, account("B", "0")).await.unwrap();
    fin::transfer(
        &c,
        TransferInput {
            from_account_id: a.id.clone(),
            to_account_id: b.id.clone(),
            date: "2026-10-01".into(),
            amount: d("40"),
            description: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        (balance(&c, &a.id).await, balance(&c, &b.id).await),
        (d("60"), d("40"))
    );
    let same = TransferInput {
        from_account_id: a.id.clone(),
        to_account_id: a.id.clone(),
        date: "2026-10-01".into(),
        amount: d("1"),
        description: None,
    };
    assert!(matches!(
        fin::transfer(&c, same).await,
        Err(CoreError::Validation(_))
    ));

    let tarifa = category(&c, "Tarifas bancárias e juros pagos").await;
    let manual = TxInput {
        bank_account_id: a.id.clone(),
        date: "2026-10-02".into(),
        kind: "saida".into(),
        amount: d("5.50"),
        description: "Tarifa".into(),
        category_id: Some(tarifa),
    };
    fin::create_transaction(&c, manual).await.unwrap();
    let list = fin::transactions(
        &c,
        TxQuery {
            bank_account_id: Some(a.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        list[0].balance_after,
        d("54.50"),
        "saldo após cada lançamento"
    );

    // apagar uma ponta da transferência apaga as duas
    let transfer_id = list
        .iter()
        .find(|t| t.origin == "transferencia")
        .unwrap()
        .id
        .clone();
    fin::delete_transaction(&c, &transfer_id).await.unwrap();
    assert_eq!(
        (balance(&c, &a.id).await, balance(&c, &b.id).await),
        (d("94.50"), d("0"))
    );
}

#[tokio::test]
async fn ofx_reconcile_and_import() {
    let (dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let acc = fin::create_account(&c, account("Banco", "0"))
        .await
        .unwrap();
    let outras = category(&c, "Outras receitas").await;
    let manual = TxInput {
        bank_account_id: acc.id.clone(),
        date: "2026-10-03".into(),
        kind: "entrada".into(),
        amount: d("150"),
        description: "Pix cliente".into(),
        category_id: Some(outras.clone()),
    };
    fin::create_transaction(&c, manual).await.unwrap();

    let path = dir.path().join("extrato.ofx");
    let body = "OFXHEADER:100\n<OFX><BANKMSGSRSV1><STMTTRNRS><STMTRS><BANKTRANLIST>\n\
        <STMTTRN><DTPOSTED>20261004<TRNAMT>150,00<FITID>F1<MEMO>PIX RECEBIDO</STMTTRN>\n\
        <STMTTRN><DTPOSTED>20261005<TRNAMT>-89,90<FITID>F2<MEMO>ENERGIA</STMTTRN>\n\
        </BANKTRANLIST></STMTRS></STMTTRNRS></BANKMSGSRSV1></OFX>";
    std::fs::write(&path, body.as_bytes()).unwrap();
    let p = path.to_string_lossy().into_owned();

    let prev = ofx::preview(&c, &acc.id, &p).await.unwrap();
    let st: Vec<&str> = prev.lines.iter().map(|l| l.status.as_str()).collect();
    assert_eq!(st, ["conciliar", "novo"]);
    assert_eq!(
        prev.lines[0].match_description.as_deref(),
        Some("Pix cliente")
    );

    let energia = category(&c, "Energia, água e internet").await;
    let req = || OfxImportRequest {
        bank_account_id: acc.id.clone(),
        path: p.clone(),
        create_new: true,
        income_category_id: Some(outras.clone()),
        expense_category_id: Some(energia.clone()),
    };
    let r = ofx::import(&c, req()).await.unwrap();
    assert_eq!((r.reconciled, r.created, r.skipped), (1, 1, 0));
    assert_eq!(balance(&c, &acc.id).await, d("60.10"));
    let r = ofx::import(&c, req()).await.unwrap();
    assert_eq!(
        (r.reconciled, r.created, r.skipped),
        (0, 0, 2),
        "reimportar não duplica"
    );
    assert!(fin::transactions(&c, TxQuery::default())
        .await
        .unwrap()
        .iter()
        .all(|t| t.reconciled));
}

#[tokio::test]
async fn cash_flow_and_dre() {
    let (_dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let acc = fin::create_account(&c, account("Banco", "1000"))
        .await
        .unwrap();
    let p = products::create(
        &c,
        ProductInput {
            code: String::new(),
            barcode: None,
            name: "Kit".into(),
            category_id: None,
            unit: None,
            price: d("100"),
            cost: Some(d("40")),
            track_stock: Some(false),
            min_stock: None,
            initial_stock: None,
        },
    )
    .await
    .unwrap();
    sales::create(
        &c,
        SaleInput {
            customer_id: None,
            seller_id: None,
            payment_method: "dinheiro".into(),
            discount: None,
            notes: None,
            sold_at: Some("2026-10-10T15:00:00Z".into()),
            tz_offset_minutes: Some(-180),
            items: vec![SaleItemInput {
                product_id: p.id.clone(),
                qty: d("1"),
                unit_price: None,
                discount: None,
            }],
        },
    )
    .await
    .unwrap();
    let aluguel = category(&c, "Aluguel").await;
    let paid = fin::create_entry(&c, entry("pagar", &aluguel, "30", "2026-10-15"))
        .await
        .unwrap();
    fin::pay_entry(
        &c,
        &paid[0].id,
        PayInput {
            date: "2026-10-15".into(),
            amount: None,
            bank_account_id: Some(acc.id.clone()),
            payment_method: None,
        },
    )
    .await
    .unwrap();
    let impostos = category(&c, "Impostos e taxas").await;
    fin::create_entry(&c, entry("pagar", &impostos, "6", "2026-10-20"))
        .await
        .unwrap(); // em aberto

    let cf = reports::cash_flow(
        &c,
        CashFlowQuery {
            from: "2026-10-01".into(),
            to: "2026-10-31".into(),
            group: Some("month".into()),
        },
    )
    .await
    .unwrap();
    assert_eq!(cf.opening_balance, d("1000"));
    assert_eq!(cf.rows.len(), 1);
    let m = &cf.rows[0];
    assert_eq!(
        (m.realized_in, m.realized_out, m.projected_out),
        (d("100"), d("30"), d("6"))
    );
    assert_eq!(
        (cf.closing_balance, cf.projected_closing),
        (d("1070"), d("1064"))
    );

    let dre = reports::dre(
        &c,
        DreQuery {
            from_month: "2026-10".into(),
            to_month: "2026-10".into(),
            tz_offset_minutes: -180,
        },
    )
    .await
    .unwrap();
    let get = |k: &str| dre.lines.iter().find(|l| l.key == k).unwrap().total;
    // receita 100 − impostos 6 = 94; − CMV 40 = 54; − aluguel 30 = 24
    assert_eq!(get("receita_bruta"), d("100"));
    assert_eq!(get("deducoes"), d("-6"));
    assert_eq!(get("custos"), d("-40"));
    assert_eq!(get("lucro_bruto"), d("54"));
    assert_eq!(get("despesas_operacionais"), d("-30"));
    assert_eq!(get("resultado"), d("24"));

    let s = fin::summary(&c, "2026-10-21").await.unwrap();
    assert_eq!(
        (s.total_balance, s.payable_overdue, s.payable_overdue_count),
        (d("1070"), d("6"), 1)
    );
}

#[tokio::test]
async fn finance_is_for_managers() {
    let (_dir, core, _me) = onboarded().await;
    create_member(&core, "caixa@padaria.com.br", "user").await;
    login_as(&core, "caixa@padaria.com.br").await;
    assert!(matches!(
        core.require(Module::Finance, Action::Read).await,
        Err(CoreError::Forbidden(_))
    ));
}
