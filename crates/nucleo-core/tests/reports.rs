//! Relatórios (com valores conferidos à mão) e planilhas (exportar/importar).

mod common;

use std::collections::BTreeMap;
use std::str::FromStr;

use common::*;
use nucleo_core::modules::customers::models::CustomerInput;
use nucleo_core::modules::customers::service as customers;
use nucleo_core::modules::products::models::{Product, ProductInput};
use nucleo_core::modules::products::service as products;
use nucleo_core::modules::reports::models::ReportQuery;
use nucleo_core::modules::reports::service as reports;
use nucleo_core::modules::sales::models::{SaleInput, SaleItemInput};
use nucleo_core::modules::sales::service as sales;
use nucleo_core::modules::spreadsheet::models::{ColumnInput, ImportRequest, SheetInput};
use nucleo_core::modules::spreadsheet::service as sheet;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, Ctx};
use rust_decimal::Decimal;
use serde_json::json;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

async fn owner(core: &Core) -> Ctx {
    core.require(Module::Reports, Action::Manage).await.unwrap()
}

async fn new_product(ctx: &Ctx, name: &str, category: &str, price: &str, cost: &str) -> Product {
    let cat = products::create_category(ctx, category).await.unwrap();
    products::create(
        ctx,
        ProductInput {
            code: String::new(),
            barcode: None,
            name: name.into(),
            category_id: Some(cat.id),
            unit: None,
            price: d(price),
            cost: Some(d(cost)),
            track_stock: None,
            min_stock: Some(d("10")),
            initial_stock: Some(d("100")),
        },
    )
    .await
    .unwrap()
}

async fn new_customer(ctx: &Ctx, name: &str, city: &str, uf: &str) -> String {
    customers::create(
        ctx,
        CustomerInput {
            name: name.into(),
            document: None,
            phone: None,
            email: None,
            city: Some(city.into()),
            state: Some(uf.into()),
            notes: None,
        },
    )
    .await
    .unwrap()
    .id
}

#[allow(clippy::too_many_arguments)]
async fn sell(
    ctx: &Ctx,
    when: &str,
    payment: &str,
    customer: Option<&str>,
    seller: Option<&str>,
    items: &[(&Product, &str)],
) -> String {
    sales::create(
        ctx,
        SaleInput {
            customer_id: customer.map(Into::into),
            seller_id: seller.map(Into::into),
            payment_method: payment.into(),
            discount: None,
            notes: None,
            sold_at: Some(when.into()),
            tz_offset_minutes: Some(-180),
            pix_provider: None,
            items: items
                .iter()
                .map(|(p, q)| SaleItemInput {
                    product_id: p.id.clone(),
                    qty: d(q),
                    unit_price: None,
                    discount: None,
                })
                .collect(),
        },
    )
    .await
    .unwrap()
    .id
}

/// Outubro/2026 em Brasília: 01/10 00:00 (-03) até 01/11 00:00.
fn october(group: &str) -> ReportQuery {
    ReportQuery {
        from: Some("2026-10-01T03:00:00.000Z".into()),
        to: Some("2026-11-01T03:00:00.000Z".into()),
        tz_offset_minutes: -180,
        group: Some(group.into()),
        limit: None,
    }
}

/// Cenário: pão (R$1, custo 0,40) e bolo (R$30, custo 12); vendas em outubro.
async fn scenario(core: &Core) -> (Product, Product, String) {
    let ctx = owner(core).await;
    let pao = new_product(&ctx, "Pão", "Pães", "1", "0.40").await;
    let bolo = new_product(&ctx, "Bolo", "Doces", "30", "12").await;
    let maria = new_customer(&ctx, "Maria", "Campinas", "SP").await;
    let joao = new_customer(&ctx, "João", "Niterói", "RJ").await;
    let caixa = create_member(core, "caixa@padaria.com.br", "user").await;

    // 01/10 10:00 local: 10 pães (10,00) pix, Maria, dono
    sell(
        &ctx,
        "2026-10-01T13:00:00Z",
        "pix",
        Some(&maria),
        None,
        &[(&pao, "10")],
    )
    .await;
    // 01/10 22:30 local = 02/10 01:30 UTC → conta no DIA 01 (hora local)
    sell(
        &ctx,
        "2026-10-02T01:30:00Z",
        "dinheiro",
        None,
        Some(&caixa.id),
        &[(&bolo, "1")],
    )
    .await;
    // 03/10: 2 bolos + 20 pães = 80,00 crédito, João, caixa
    sell(
        &ctx,
        "2026-10-03T15:00:00Z",
        "credito",
        Some(&joao),
        Some(&caixa.id),
        &[(&bolo, "2"), (&pao, "20")],
    )
    .await;
    // venda cancelada em 03/10 (fica fora dos totais)
    let canceled = sell(
        &ctx,
        "2026-10-03T16:00:00Z",
        "pix",
        Some(&maria),
        None,
        &[(&bolo, "1")],
    )
    .await;
    sales::cancel(&ctx, &canceled, "erro de digitação")
        .await
        .unwrap();
    // fora do período (setembro)
    sell(
        &ctx,
        "2026-09-30T12:00:00Z",
        "pix",
        None,
        None,
        &[(&bolo, "5")],
    )
    .await;
    (pao, bolo, caixa.id)
}

#[tokio::test]
async fn sales_summary_by_local_day_and_payment() {
    let (_dir, core, _me) = onboarded().await;
    scenario(&core).await;
    let ctx = owner(&core).await;
    let s = reports::sales_summary(&ctx, october("day")).await.unwrap();

    // 10 + 30 + 80 = 120 ; custo 4 + 12 + (24 + 8) = 48 ; lucro 72 ; margem 60%
    let t = &s.totals;
    assert_eq!(
        (t.count, t.total, t.cost, t.profit),
        (3, d("120"), d("48"), d("72"))
    );
    assert_eq!((t.margin_pct, t.avg_ticket), (d("60"), d("40")));
    assert_eq!((t.canceled_count, t.canceled_total), (1, d("30")));
    assert_eq!(t.items_qty, d("33"));

    assert_eq!(
        s.series.len(),
        31,
        "todos os dias de outubro, mesmo sem venda"
    );
    let day = |k: &str| s.series.iter().find(|p| p.period == k).unwrap();
    assert_eq!(
        (day("2026-10-01").count, day("2026-10-01").total),
        (2, d("40"))
    );
    assert_eq!(day("2026-10-02").count, 0);
    assert_eq!(day("2026-10-03").total, d("80"));

    assert_eq!(s.by_payment[0].method, "credito");
    assert_eq!(s.by_payment[0].share_pct, d("66.7"));
    assert_eq!(s.by_hour.len(), 24);
    assert_eq!(s.by_hour[22].count, 1, "22h local");

    let monthly = reports::sales_summary(&ctx, october("month"))
        .await
        .unwrap();
    assert_eq!(monthly.series.len(), 1);
    assert_eq!(monthly.series[0].period, "2026-10");
}

#[tokio::test]
async fn products_abc_stock_customers_sellers() {
    let (_dir, core, _me) = onboarded().await;
    let (pao, _bolo, caixa_id) = scenario(&core).await;
    let ctx = owner(&core).await;

    let p = reports::products(&ctx, october("day")).await.unwrap();
    // bolo 90 (75%) → A ; pão 30 (25%) → B (acumulado antes = 75% < 80% → A também)
    assert_eq!(p.products[0].name, "Bolo");
    assert_eq!(
        (p.products[0].revenue, p.products[0].share_pct),
        (d("90"), d("75"))
    );
    assert_eq!(p.products[0].abc, "A");
    assert_eq!(
        (p.products[1].name.as_str(), p.products[1].abc.as_str()),
        ("Pão", "A")
    );
    assert_eq!(p.products[1].margin_pct, d("60"));
    assert_eq!(p.categories[0].category, "Doces");

    let st = reports::stock(&ctx, october("day")).await.unwrap();
    let row = st.rows.iter().find(|r| r.product_id == pao.id).unwrap();
    // 100 − 10 − 20 = 70 em estoque; 30 vendidos em 31 dias → ~72 dias de cobertura
    assert_eq!((row.stock, row.sold_qty), (d("70"), d("30")));
    assert_eq!(row.days_of_cover, Some(d("72")));
    assert_eq!(row.cost_value, d("28"));

    let c = reports::customers(&ctx, october("day")).await.unwrap();
    assert_eq!(c.by_state[0].state, "RJ");
    assert_eq!(c.by_state[0].total, d("80"));
    assert_eq!(c.by_state.len(), 2);
    assert_eq!((c.anonymous_count, c.anonymous_total), (1, d("30")));
    assert_eq!(c.top_customers[0].name, "João");

    let sellers = reports::sellers(&ctx, october("day")).await.unwrap();
    let caixa = sellers.iter().find(|s| s.seller_id == caixa_id).unwrap();
    assert_eq!(
        (caixa.count, caixa.total, caixa.share_pct),
        (2, d("110"), d("91.7"))
    );
    assert_eq!(caixa.customers_served, 1);
    let dono = sellers.iter().find(|s| s.seller_id != caixa_id).unwrap();
    assert_eq!((dono.count, dono.canceled_count), (1, 1));
}

#[tokio::test]
async fn reports_are_for_managers() {
    let (_dir, core, _me) = onboarded().await;
    create_member(&core, "caixa@padaria.com.br", "user").await;
    login_as(&core, "caixa@padaria.com.br").await;
    assert!(core.require(Module::Reports, Action::Read).await.is_err());
}

#[tokio::test]
async fn export_then_import_roundtrip() {
    let (dir, core, _me) = onboarded().await;
    let ctx = owner(&core).await;

    // exportar
    let path = dir.path().join("relatorio.xlsx");
    let sheets = vec![SheetInput {
        name: "Produtos/Out".into(),
        columns: vec![
            ColumnInput {
                label: "Código".into(),
                kind: "text".into(),
            },
            ColumnInput {
                label: "Nome".into(),
                kind: "text".into(),
            },
            ColumnInput {
                label: "Preço".into(),
                kind: "money".into(),
            },
            ColumnInput {
                label: "Estoque".into(),
                kind: "number".into(),
            },
            ColumnInput {
                label: "Margem".into(),
                kind: "percent".into(),
            },
            ColumnInput {
                label: "Data".into(),
                kind: "date".into(),
            },
        ],
        rows: vec![
            vec![
                json!("0001"),
                json!("Pão"),
                json!("0.80"),
                json!("12.5"),
                json!("60"),
                json!("2026-10-01"),
            ],
            vec![
                json!("0002"),
                json!("Bolo"),
                json!(30),
                json!(null),
                json!("40.5"),
                json!("x"),
            ],
        ],
    }];
    sheet::export_xlsx(&path, &sheets).unwrap();
    let back = sheet::preview(&path).unwrap();
    assert_eq!(back.headers[2], "Preço");
    assert_eq!(back.total_rows, 2);
    assert_eq!(back.sample[0][2], "0.8");

    // importar CSV em formato brasileiro (;  vírgula decimal, Latin-1 como o Excel salva)
    let csv_path = dir.path().join("produtos.csv");
    let csv = "Código;Descrição;Grupo;UN;Preço Venda;Custo;Estoque\n\
               P1;Pão francês;Pães;un;0,80;0,35;100\n\
               P2;Queijo minas;Frios;kg;R$ 39,90;25;2,5\n\
               P3;;Pães;un;1;0;1\n\
               P4;Coxinha;Salgados;caixa;5;2;10\n";
    let latin1: Vec<u8> = csv.chars().map(|c| c as u32 as u8).collect();
    std::fs::write(&csv_path, latin1).unwrap();

    let prev = sheet::preview(&csv_path).unwrap();
    assert_eq!(prev.headers[1], "Descrição");
    let mapping: BTreeMap<String, usize> = prev.suggested.clone();
    assert_eq!(mapping["name"], 1);
    assert_eq!(mapping["price"], 4);

    let req = |update: bool| ImportRequest {
        path: csv_path.to_string_lossy().into_owned(),
        mapping: mapping.clone(),
        update_existing: update,
    };
    let r = sheet::import_products(&ctx, req(false)).await.unwrap();
    assert_eq!((r.created, r.updated, r.skipped), (2, 0, 0));
    let rows: Vec<usize> = r.errors.iter().map(|e| e.row).collect();
    assert_eq!(rows, [4, 5], "nome vazio e unidade desconhecida");

    let list = products::list(&ctx).await.unwrap();
    let queijo = list.iter().find(|p| p.code == "P2").unwrap();
    assert_eq!(
        (queijo.price, queijo.stock, queijo.unit.as_str()),
        (d("39.9"), d("2.5"), "kg")
    );
    assert_eq!(queijo.category_name.as_deref(), Some("Frios"));

    // de novo: sem atualizar, ignora; atualizando, ajusta o estoque pela contagem
    let r = sheet::import_products(&ctx, req(false)).await.unwrap();
    assert_eq!((r.created, r.skipped), (0, 2));
    let r = sheet::import_products(&ctx, req(true)).await.unwrap();
    assert_eq!(r.updated, 2);
}
