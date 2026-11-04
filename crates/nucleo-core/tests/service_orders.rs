//! Ordens de serviço: criação, itens, fluxo de status, linha do tempo, faturamento
//! (venda + estoque + financeiro na mesma transação), cancelamento e permissões.

mod common;

use std::str::FromStr;

use common::*;
use nucleo_core::modules::customers::models::CustomerInput;
use nucleo_core::modules::customers::service as customers;
use nucleo_core::modules::finance::models::EntryQuery;
use nucleo_core::modules::finance::service as fin;
use nucleo_core::modules::products::models::ProductInput;
use nucleo_core::modules::products::service as products;
use nucleo_core::modules::sales::service as sales;
use nucleo_core::modules::service_orders::models::*;
use nucleo_core::modules::service_orders::service as os;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, CoreError, Ctx};
use rust_decimal::Decimal;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

async fn ctx(core: &Core) -> Ctx {
    core.require(Module::ServiceOrders, Action::Manage)
        .await
        .unwrap()
}

struct Fixture {
    customer: String,
    labor: String,
    part: String,
}

async fn fixture(c: &Ctx) -> Fixture {
    let customer = customers::create(
        c,
        CustomerInput {
            name: "Oficina do Zé".into(),
            document: None,
            phone: Some("(11) 99999-0000".into()),
            email: None,
            city: Some("São Paulo".into()),
            state: Some("SP".into()),
            notes: None,
        },
    )
    .await
    .unwrap()
    .id;
    let labor = products::create(
        c,
        ProductInput {
            code: String::new(),
            barcode: None,
            name: "Mão de obra".into(),
            category_id: None,
            unit: Some("h".into()),
            price: d("120"),
            cost: None,
            track_stock: Some(false),
            min_stock: None,
            initial_stock: None,
        },
    )
    .await
    .unwrap()
    .id;
    let part = products::create(
        c,
        ProductInput {
            code: String::new(),
            barcode: None,
            name: "Correia".into(),
            category_id: None,
            unit: Some("un".into()),
            price: d("45.50"),
            cost: Some(d("20")),
            track_stock: Some(true),
            min_stock: None,
            initial_stock: Some(d("5")),
        },
    )
    .await
    .unwrap()
    .id;
    Fixture {
        customer,
        labor,
        part,
    }
}

fn item(product_id: &str, qty: &str) -> OsItemInput {
    OsItemInput {
        product_id: product_id.into(),
        description: None,
        qty: d(qty),
        unit_price: None,
        discount: None,
    }
}

fn input(f: &Fixture) -> OsInput {
    OsInput {
        customer_id: f.customer.clone(),
        equipment: Some("Motor estacionário".into()),
        reported: "Não liga".into(),
        technician_id: None,
        scheduled_date: Some("2026-11-05".into()),
        due_date: Some("2026-11-12".into()),
        items: vec![item(&f.labor, "1.5")],
    }
}

async fn stock(c: &Ctx, id: &str) -> Decimal {
    products::list(c)
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.id == id)
        .unwrap()
        .stock
}

#[tokio::test]
async fn create_items_status_and_timeline() {
    let (_dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let f = fixture(&c).await;

    let o = os::create(&c, input(&f)).await.unwrap();
    assert_eq!(o.number, 1);
    assert_eq!(o.status, "aberta");
    assert_eq!(o.due_date.as_deref(), Some("2026-11-12"));
    assert_eq!(o.total, d("180"));
    assert_eq!(o.items.as_ref().unwrap()[0].description, "Mão de obra");
    assert!(o.items.as_ref().unwrap()[0].is_service);
    assert_eq!(os::create(&c, input(&f)).await.unwrap().number, 2);

    // sem relato → validação
    let mut bad = input(&f);
    bad.reported = " ".into();
    assert!(matches!(
        os::create(&c, bad).await,
        Err(CoreError::Validation(_))
    ));

    // itens: mão de obra + 2 correias com desconto de 1,00; desconto geral de 10
    let mut part = item(&f.part, "2");
    part.discount = Some(d("1"));
    let o = os::set_items(&c, &o.id, vec![item(&f.labor, "1.5"), part])
        .await
        .unwrap();
    assert_eq!(o.subtotal, d("270"));
    let patch = OsPatch {
        discount: Some(d("10")),
        diagnosis: Some(Some("Correia partida".into())),
        ..Default::default()
    };
    let o = os::update(&c, &o.id, patch).await.unwrap();
    assert_eq!(o.total, d("260"));
    assert_eq!(o.diagnosis.as_deref(), Some("Correia partida"));
    assert_eq!(o.item_count, 2);

    // status livre entre os de trabalho; faturar/cancelar não passam por aqui
    let o = os::change_status(&c, &o.id, "em_andamento", None)
        .await
        .unwrap();
    assert!(matches!(
        os::change_status(&c, &o.id, "faturada", None).await,
        Err(CoreError::Validation(_))
    ));
    let o = os::add_note(&c, &o.id, "Peça encomendada").await.unwrap();
    let o = os::change_status(&c, &o.id, "concluida", Some("Pronto".into()))
        .await
        .unwrap();
    assert!(o.finished_at.is_some());
    let kinds: Vec<&str> = o
        .events
        .as_ref()
        .unwrap()
        .iter()
        .map(|e| e.kind.as_str())
        .collect();
    assert_eq!(kinds, ["criada", "status", "nota", "status"]);

    let list = os::list(
        &c,
        OsQuery {
            status: Some("concluida".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].total, d("260"));
}

#[tokio::test]
async fn invoice_creates_sale_moves_stock_and_finance() {
    let (_dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let f = fixture(&c).await;
    let o = os::create(&c, input(&f)).await.unwrap();
    let o = os::set_items(&c, &o.id, vec![item(&f.labor, "1.5"), item(&f.part, "2")])
        .await
        .unwrap();
    let fiado = InvoiceInput {
        payment_method: "fiado".into(),
        seller_id: None,
        tz_offset_minutes: Some(-180),
    };

    // só depois de concluída
    assert!(matches!(
        os::invoice(&c, &o.id, fiado.clone()).await,
        Err(CoreError::BusinessRule(_))
    ));
    os::change_status(&c, &o.id, "concluida", None)
        .await
        .unwrap();

    // estoque insuficiente → nada é gravado (transação única)
    os::set_items(&c, &o.id, vec![item(&f.part, "9")])
        .await
        .unwrap();
    assert!(os::invoice(&c, &o.id, fiado.clone()).await.is_err());
    let still = os::get(&c, &o.id).await.unwrap();
    assert_eq!(still.status, "concluida");
    assert!(still.sale_id.is_none());

    os::set_items(&c, &o.id, vec![item(&f.labor, "1.5"), item(&f.part, "2")])
        .await
        .unwrap();
    let o = os::invoice(&c, &o.id, fiado).await.unwrap();
    assert_eq!(o.status, "faturada");
    assert_eq!(o.sale_number, Some(1));
    let sale = sales::get(&c, o.sale_id.as_deref().unwrap()).await.unwrap();
    assert_eq!(sale.total, d("271"));
    assert_eq!(sale.notes.as_deref(), Some("OS nº 1"));
    assert_eq!(stock(&c, &f.part).await, d("3"), "peças saem do estoque");

    let receber = fin::entries(
        &c,
        EntryQuery {
            kind: Some("receber".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(receber.len(), 1);
    assert_eq!(receber[0].amount, d("271"));

    // faturada fica travada
    assert!(matches!(
        os::set_items(&c, &o.id, vec![]).await,
        Err(CoreError::BusinessRule(_))
    ));
    assert!(matches!(
        os::cancel(&c, &o.id, "desistiu").await,
        Err(CoreError::BusinessRule(_))
    ));
}

#[tokio::test]
async fn cancel_rules() {
    let (_dir, core, _me) = onboarded().await;
    let c = ctx(&core).await;
    let f = fixture(&c).await;
    let o = os::create(&c, input(&f)).await.unwrap();
    assert!(matches!(
        os::cancel(&c, &o.id, "").await,
        Err(CoreError::Validation(_))
    ));
    let o = os::cancel(&c, &o.id, "Cliente desistiu").await.unwrap();
    assert_eq!(o.status, "cancelada");
    assert_eq!(o.canceled_reason.as_deref(), Some("Cliente desistiu"));
    assert!(matches!(
        os::change_status(&c, &o.id, "aberta", None).await,
        Err(CoreError::BusinessRule(_))
    ));
    assert!(matches!(
        os::cancel(&c, &o.id, "de novo").await,
        Err(CoreError::BusinessRule(_))
    ));
}

#[tokio::test]
async fn permissions_by_role() {
    let (_dir, core, _me) = onboarded().await;
    create_member(&core, "tec@padaria.com.br", "user").await;
    login_as(&core, "tec@padaria.com.br").await;
    assert!(core
        .require(Module::ServiceOrders, Action::Write)
        .await
        .is_ok());
    assert!(matches!(
        core.require(Module::ServiceOrders, Action::Manage).await,
        Err(CoreError::Forbidden(_))
    ));
}
