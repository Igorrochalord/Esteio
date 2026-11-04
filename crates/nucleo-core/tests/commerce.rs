//! Produtos, estoque, clientes e vendas.

mod common;

use std::str::FromStr;

use common::*;
use nucleo_core::modules::customers::models::{CustomerInput, CustomerPatch};
use nucleo_core::modules::customers::service as customers;
use nucleo_core::modules::products::models::{
    MovementQuery, Product, ProductInput, ProductPatch, StockInput,
};
use nucleo_core::modules::products::service as products;
use nucleo_core::modules::sales::models::{SaleInput, SaleItemInput, SaleQuery};
use nucleo_core::modules::sales::service as sales;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, CoreError, Ctx};
use rust_decimal::Decimal;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

async fn manager(core: &Core) -> Ctx {
    core.require(Module::Products, Action::Manage)
        .await
        .unwrap()
}

fn product(name: &str, price: &str, cost: &str, stock: Option<&str>) -> ProductInput {
    ProductInput {
        code: String::new(),
        barcode: None,
        name: name.into(),
        category_id: None,
        unit: None,
        price: d(price),
        cost: Some(d(cost)),
        track_stock: None,
        min_stock: Some(d("5")),
        initial_stock: stock.map(d),
    }
}

fn sale(items: Vec<(&Product, &str)>, payment: &str) -> SaleInput {
    SaleInput {
        customer_id: None,
        seller_id: None,
        payment_method: payment.into(),
        discount: None,
        notes: None,
        sold_at: None,
        items: items
            .into_iter()
            .map(|(p, q)| SaleItemInput {
                product_id: p.id.clone(),
                qty: d(q),
                unit_price: None,
                discount: None,
            })
            .collect(),
    }
}

#[tokio::test]
async fn products_codes_categories_and_stock() {
    let (_dir, core, _me) = onboarded().await;
    let ctx = manager(&core).await;

    let cat = products::create_category(&ctx, "Pães").await.unwrap();
    let mut input = product("Pão francês", "0.80", "0.35", Some("100"));
    input.category_id = Some(cat.id.clone());
    let pao = products::create(&ctx, input).await.unwrap();
    assert_eq!(pao.code, "0001");
    assert_eq!(pao.stock, d("100"));
    assert_eq!(pao.category_name.as_deref(), Some("Pães"));
    let bolo = products::create(&ctx, product("Bolo", "32", "14", None))
        .await
        .unwrap();
    assert_eq!(bolo.code, "0002");
    assert!(bolo.below_min, "sem estoque e mínimo 5");

    // código duplicado (sem diferenciar maiúsculas)
    let mut dup = product("Outro", "1", "0", None);
    dup.code = "0001".into();
    assert!(matches!(
        products::create(&ctx, dup).await,
        Err(CoreError::Conflict(_))
    ));

    // edição de uma célula
    let patch = ProductPatch {
        price: Some(d("0.90")),
        ..Default::default()
    };
    assert_eq!(
        products::update(&ctx, &pao.id, patch).await.unwrap().price,
        d("0.9")
    );

    // entrada com custo atualiza o custo; saída maior que o saldo é barrada; ajuste por contagem
    let entrada = StockInput {
        product_id: bolo.id.clone(),
        kind: "entrada".into(),
        qty: d("10"),
        unit_cost: Some(d("15")),
        note: None,
    };
    let bolo = products::move_stock(&ctx, entrada).await.unwrap();
    assert_eq!((bolo.stock, bolo.cost), (d("10"), Some(d("15"))));
    let demais = StockInput {
        product_id: bolo.id.clone(),
        kind: "saida".into(),
        qty: d("11"),
        unit_cost: None,
        note: None,
    };
    assert!(matches!(
        products::move_stock(&ctx, demais).await,
        Err(CoreError::BusinessRule(_))
    ));
    let contagem = StockInput {
        product_id: bolo.id.clone(),
        kind: "ajuste".into(),
        qty: d("7"),
        unit_cost: None,
        note: Some("contagem".into()),
    };
    assert_eq!(
        products::move_stock(&ctx, contagem).await.unwrap().stock,
        d("7")
    );

    let movs = products::movements(
        &ctx,
        MovementQuery {
            product_id: Some(bolo.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(movs.len(), 2);
    assert_eq!((movs[0].kind.as_str(), movs[0].qty), ("ajuste", d("-3")));

    // categoria com produto não pode ser excluída
    assert!(matches!(
        products::delete_category(&ctx, &cat.id).await,
        Err(CoreError::BusinessRule(_))
    ));

    // movimentações são imutáveis
    let upd = sqlx::query("UPDATE stock_movements SET qty_milli = 0")
        .execute(&core.pool())
        .await;
    assert!(upd.is_err());
}

#[tokio::test]
async fn fractional_quantities() {
    let (_dir, core, _me) = onboarded().await;
    let ctx = manager(&core).await;
    let mut input = product("Queijo", "39.90", "25", Some("2.5"));
    input.unit = Some("kg".into());
    let queijo = products::create(&ctx, input).await.unwrap();
    let ctx = core.require(Module::Sales, Action::Write).await.unwrap();
    let s = sales::create(&ctx, sale(vec![(&queijo, "1.25")], "pix"))
        .await
        .unwrap();
    assert_eq!(s.total, d("49.88")); // 1,25 × 39,90 = 49,875 → 49,88
    let ctx = manager(&core).await;
    let after = products::list(&ctx).await.unwrap();
    assert_eq!(after[0].stock, d("1.25"));
}

#[tokio::test]
async fn sale_totals_stock_and_cancel() {
    let (_dir, core, _me) = onboarded().await;
    let ctx = manager(&core).await;
    let pao = products::create(&ctx, product("Pão", "0.80", "0.35", Some("100")))
        .await
        .unwrap();
    let cafe = products::create(&ctx, product("Café", "18.90", "11.20", Some("3")))
        .await
        .unwrap();

    let mut input = sale(vec![(&pao, "10"), (&cafe, "2")], "dinheiro");
    input.items[1].discount = Some(d("1.80"));
    input.discount = Some(d("2"));
    let s = sales::create(&ctx, input).await.unwrap();
    // 10×0,80 = 8,00 ; 2×18,90 = 37,80 − 1,80 = 36,00 ; subtotal 44,00 − 2 = 42,00
    assert_eq!((s.number, s.subtotal, s.total), (1, d("44"), d("42")));
    // custo 10×0,35 + 2×11,20 = 25,90 → lucro 16,10
    assert_eq!((s.cost_total, s.profit), (Some(d("25.9")), Some(d("16.1"))));
    assert_eq!(s.items.as_ref().unwrap().len(), 2);

    let stock = |list: &Vec<Product>, id: &str| list.iter().find(|p| p.id == id).unwrap().stock;
    let list = products::list(&ctx).await.unwrap();
    assert_eq!(
        (stock(&list, &pao.id), stock(&list, &cafe.id)),
        (d("90"), d("1"))
    );

    // estoque insuficiente bloqueia a venda inteira (nada é gravado)
    let falta = sales::create(&ctx, sale(vec![(&pao, "1"), (&cafe, "2")], "pix")).await;
    assert!(matches!(falta, Err(CoreError::BusinessRule(_))));
    let list = products::list(&ctx).await.unwrap();
    assert_eq!(stock(&list, &pao.id), d("90"));

    // fiado exige cliente; forma inválida é recusada
    assert!(matches!(
        sales::create(&ctx, sale(vec![(&pao, "1")], "fiado")).await,
        Err(CoreError::Validation(_))
    ));
    assert!(matches!(
        sales::create(&ctx, sale(vec![(&pao, "1")], "cheque")).await,
        Err(CoreError::Validation(_))
    ));

    // cancelar devolve o estoque e mantém a venda no histórico
    let canceled = sales::cancel(&ctx, &s.id, "cliente desistiu")
        .await
        .unwrap();
    assert_eq!(canceled.status, "cancelada");
    let list = products::list(&ctx).await.unwrap();
    assert_eq!(
        (stock(&list, &pao.id), stock(&list, &cafe.id)),
        (d("100"), d("3"))
    );
    assert!(matches!(
        sales::cancel(&ctx, &s.id, "de novo").await,
        Err(CoreError::BusinessRule(_))
    ));
    let next = sales::create(&ctx, sale(vec![(&pao, "1")], "pix"))
        .await
        .unwrap();
    assert_eq!(next.number, 2);
    assert_eq!(
        sales::list(&ctx, SaleQuery::default()).await.unwrap().len(),
        2
    );
}

#[tokio::test]
async fn operator_sells_but_does_not_see_cost_or_backdate() {
    let (_dir, core, _me) = onboarded().await;
    let ctx = manager(&core).await;
    let pao = products::create(&ctx, product("Pão", "0.80", "0.35", Some("10")))
        .await
        .unwrap();
    create_member(&core, "caixa@padaria.com.br", "user").await;

    login_as(&core, "caixa@padaria.com.br").await;
    let caixa = core.require(Module::Sales, Action::Write).await.unwrap();
    let list = products::list(&caixa).await.unwrap();
    assert_eq!(list[0].cost, None, "operador não vê custo");
    let s = sales::create(&caixa, sale(vec![(&pao, "2")], "debito"))
        .await
        .unwrap();
    assert_eq!(
        (s.profit, s.seller_id.as_str()),
        (None, caixa.user.id.as_str())
    );

    let mut old = sale(vec![(&pao, "1")], "pix");
    old.sold_at = Some("2026-01-10".into());
    assert!(matches!(
        sales::create(&caixa, old).await,
        Err(CoreError::Validation(_))
    ));
    // cancelar e editar produto são de quem gerencia
    assert!(matches!(
        core.require(Module::Sales, Action::Manage).await,
        Err(CoreError::Forbidden(_))
    ));
    assert!(matches!(
        core.require(Module::Products, Action::Write).await,
        Err(CoreError::Forbidden(_))
    ));
}

#[tokio::test]
async fn customers_validation_and_fiado() {
    let (_dir, core, _me) = onboarded().await;
    let ctx = manager(&core).await;
    let bad = CustomerInput {
        name: "Maria".into(),
        document: Some("123".into()),
        phone: None,
        email: Some("x".into()),
        city: None,
        state: Some("ZZ".into()),
        notes: None,
    };
    match customers::create(&ctx, bad).await {
        Err(CoreError::Validation(f)) => {
            assert!(
                f.contains_key("document") && f.contains_key("email") && f.contains_key("state")
            )
        }
        other => panic!("esperava Validation, veio {other:?}"),
    }
    let maria = customers::create(
        &ctx,
        CustomerInput {
            name: "Maria Silva".into(),
            document: Some("529.982.247-25".into()),
            phone: Some("(11) 99999-0000".into()),
            email: None,
            city: Some("Campinas".into()),
            state: Some("sp".into()),
            notes: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        (maria.document.as_deref(), maria.state.as_deref()),
        (Some("52998224725"), Some("SP"))
    );

    let patch = CustomerPatch {
        city: Some(None),
        ..Default::default()
    };
    assert_eq!(
        customers::update(&ctx, &maria.id, patch)
            .await
            .unwrap()
            .city,
        None
    );

    let pao = products::create(&ctx, product("Pão", "0.80", "0.35", Some("10")))
        .await
        .unwrap();
    let mut fiado = sale(vec![(&pao, "5")], "fiado");
    fiado.customer_id = Some(maria.id.clone());
    let s = sales::create(&ctx, fiado).await.unwrap();
    assert_eq!(s.customer_name.as_deref(), Some("Maria Silva"));
    let por_cliente = SaleQuery {
        customer_id: Some(maria.id.clone()),
        ..Default::default()
    };
    assert_eq!(sales::list(&ctx, por_cliente).await.unwrap().len(), 1);
}
