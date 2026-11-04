//! Relatórios financeiros: fluxo de caixa (realizado + previsto) e DRE mensal.

use std::collections::BTreeMap;

use chrono::{Months, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::money::from_cents;
use crate::Ctx;

use super::service::parse_date;

#[derive(Debug, Deserialize)]
pub struct CashFlowQuery {
    /// AAAA-MM-DD (inclusivo)
    pub from: String,
    pub to: String,
    /// day | month
    pub group: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CashFlowRow {
    pub period: String,
    pub realized_in: Decimal,
    pub realized_out: Decimal,
    pub projected_in: Decimal,
    pub projected_out: Decimal,
    /// saldo acumulado só com o realizado
    pub balance: Decimal,
    /// saldo acumulado com realizado + previsto
    pub projected_balance: Decimal,
}

#[derive(Debug, Serialize)]
pub struct CashFlow {
    pub opening_balance: Decimal,
    pub rows: Vec<CashFlowRow>,
    pub closing_balance: Decimal,
    pub projected_closing: Decimal,
}

pub async fn cash_flow(ctx: &Ctx, q: CashFlowQuery) -> CoreResult<CashFlow> {
    let (from, to) = match (parse_date(&q.from), parse_date(&q.to)) {
        (Some(f), Some(t)) if f <= t => (f, t),
        _ => return Err(CoreError::field("to", "Período inválido")),
    };
    if (to - from).num_days() > 3700 {
        return Err(CoreError::field("to", "Período longo demais"));
    }
    let monthly = q.group.as_deref() == Some("month");
    let key_len: i64 = if monthly { 7 } else { 10 };
    let (f, t) = (
        from.format("%Y-%m-%d").to_string(),
        to.format("%Y-%m-%d").to_string(),
    );
    let mut conn = ctx.db.acquire().await?;

    let opening: i64 = sqlx::query_scalar(
        "SELECT coalesce((SELECT sum(initial_balance_cents) FROM bank_accounts WHERE deleted_at IS NULL), 0)
              + coalesce((SELECT sum(t.amount_cents) FROM bank_transactions t WHERE t.deleted_at IS NULL AND t.date < ?), 0)",
    )
    .bind(&f)
    .fetch_one(&mut *conn)
    .await?;
    // transferências somam zero no total e só inflariam entradas/saídas: ficam fora
    let realized: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT substr(date, 1, ?1) AS k,
                coalesce(sum(CASE WHEN amount_cents > 0 THEN amount_cents END), 0),
                coalesce(sum(CASE WHEN amount_cents < 0 THEN -amount_cents END), 0)
         FROM bank_transactions
         WHERE deleted_at IS NULL AND transfer_group IS NULL AND date BETWEEN ?2 AND ?3
         GROUP BY k",
    )
    .bind(key_len)
    .bind(&f)
    .bind(&t)
    .fetch_all(&mut *conn)
    .await?;
    let projected: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT substr(due_date, 1, ?1) AS k,
                coalesce(sum(CASE WHEN kind = 'receber' THEN amount_cents END), 0),
                coalesce(sum(CASE WHEN kind = 'pagar' THEN amount_cents END), 0)
         FROM fin_entries
         WHERE deleted_at IS NULL AND status = 'aberto' AND due_date BETWEEN ?2 AND ?3
         GROUP BY k",
    )
    .bind(key_len)
    .bind(&f)
    .bind(&t)
    .fetch_all(&mut *conn)
    .await?;

    let mut map: BTreeMap<String, [i64; 4]> = BTreeMap::new();
    let mut d = from;
    while d <= to {
        map.entry(
            d.format(if monthly { "%Y-%m" } else { "%Y-%m-%d" })
                .to_string(),
        )
        .or_default();
        d = d.succ_opt().unwrap_or(to + chrono::Days::new(1));
    }
    for (k, i, o) in realized {
        let e = map.entry(k).or_default();
        e[0] += i;
        e[1] += o;
    }
    for (k, i, o) in projected {
        let e = map.entry(k).or_default();
        e[2] += i;
        e[3] += o;
    }
    let (mut bal, mut proj) = (opening, opening);
    let rows = map
        .into_iter()
        .map(|(period, [ri, ro, pi, po])| {
            bal += ri - ro;
            proj += ri - ro + pi - po;
            CashFlowRow {
                period,
                realized_in: from_cents(ri),
                realized_out: from_cents(ro),
                projected_in: from_cents(pi),
                projected_out: from_cents(po),
                balance: from_cents(bal),
                projected_balance: from_cents(proj),
            }
        })
        .collect();
    Ok(CashFlow {
        opening_balance: from_cents(opening),
        rows,
        closing_balance: from_cents(bal),
        projected_closing: from_cents(proj),
    })
}

#[derive(Debug, Deserialize)]
pub struct DreQuery {
    /// AAAA-MM
    pub from_month: String,
    pub to_month: String,
    /// fuso da tela (vendas são agrupadas pelo mês local)
    #[serde(default)]
    pub tz_offset_minutes: i32,
}

#[derive(Debug, Serialize)]
pub struct DreLine {
    pub key: String,
    pub label: String,
    /// group = linha de valores; subtotal = resultado parcial; info = fora do resultado
    pub kind: String,
    /// com sinal: receitas positivas, custos e despesas negativos
    pub values: Vec<Decimal>,
    pub total: Decimal,
}

#[derive(Debug, Serialize)]
pub struct Dre {
    pub months: Vec<String>,
    pub lines: Vec<DreLine>,
}

fn month(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&format!("{}-01", s.trim()), "%Y-%m-%d").ok()
}

// PLANO: DRE é relatório avançado (Platinum em diante); hoje liberado.
pub async fn dre(ctx: &Ctx, q: DreQuery) -> CoreResult<Dre> {
    let (from, to) = match (month(&q.from_month), month(&q.to_month)) {
        (Some(f), Some(t)) if f <= t => (f, t),
        _ => return Err(CoreError::field("to_month", "Período inválido")),
    };
    let mut months = Vec::new();
    let mut m = from;
    while m <= to && months.len() < 60 {
        months.push(m.format("%Y-%m").to_string());
        m = m
            .checked_add_months(Months::new(1))
            .unwrap_or(to + chrono::Days::new(1));
    }
    let (fm, tm) = (months[0].clone(), months[months.len() - 1].clone());
    let idx = |k: &str| months.iter().position(|x| x == k);
    let tz = format!("{:+} minutes", q.tz_offset_minutes.clamp(-840, 840));
    let mut conn = ctx.db.acquire().await?;
    // grupo → valores com sinal por mês
    let mut g: BTreeMap<&'static str, Vec<i64>> = BTreeMap::new();
    let mut add = |group: &str, i: usize, cents: i64| {
        let key: &'static str = match group {
            "receita_vendas" => "receita_vendas",
            "outras_receitas" => "outras_receitas",
            "deducoes" => "deducoes",
            "impostos" => "impostos",
            "custos" => "custos",
            "despesas_operacionais" => "despesas_operacionais",
            "despesas_pessoal" => "despesas_pessoal",
            "despesas_financeiras" => "despesas_financeiras",
            "receitas_financeiras" => "receitas_financeiras",
            "investimentos" => "investimentos",
            "cmv" => "cmv",
            "compras_estoque" => "compras_estoque",
            _ => "despesas_operacionais",
        };
        g.entry(key).or_insert_with(|| vec![0; months.len()])[i] += cents;
    };

    // vendas concluídas (mês local): receita e CMV
    let sales: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT strftime('%Y-%m', sold_at, ?1) AS m, sum(total_cents), sum(cost_total_cents)
         FROM sales WHERE status = 'concluida' AND deleted_at IS NULL
         GROUP BY m HAVING m BETWEEN ?2 AND ?3",
    )
    .bind(&tz)
    .bind(&fm)
    .bind(&tm)
    .fetch_all(&mut *conn)
    .await?;
    for (mk, total, cost) in sales {
        if let Some(i) = idx(&mk) {
            add("receita_vendas", i, total);
            add("cmv", i, -cost);
        }
    }
    // títulos (competência = emissão), exceto os de venda (já contados acima)
    let entries: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT substr(e.issue_date, 1, 7) AS m, c.dre_group, e.kind, sum(e.amount_cents)
         FROM fin_entries e JOIN fin_categories c ON c.id = e.category_id
         WHERE e.deleted_at IS NULL AND e.status <> 'cancelado' AND e.sale_id IS NULL
         GROUP BY m, c.dre_group, e.kind HAVING m BETWEEN ?1 AND ?2",
    )
    .bind(&fm)
    .bind(&tm)
    .fetch_all(&mut *conn)
    .await?;
    for (mk, group, kind, cents) in entries {
        if let Some(i) = idx(&mk) {
            add(&group, i, if kind == "receber" { cents } else { -cents });
        }
    }
    // lançamentos avulsos com categoria (valor já tem sinal)
    let manual: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT substr(t.date, 1, 7) AS m, c.dre_group, sum(t.amount_cents)
         FROM bank_transactions t JOIN fin_categories c ON c.id = t.category_id
         WHERE t.deleted_at IS NULL AND t.entry_id IS NULL AND t.sale_id IS NULL AND t.transfer_group IS NULL
         GROUP BY m, c.dre_group HAVING m BETWEEN ?1 AND ?2",
    )
    .bind(&fm)
    .bind(&tm)
    .fetch_all(&mut *conn)
    .await?;
    for (mk, group, cents) in manual {
        if let Some(i) = idx(&mk) {
            add(&group, i, cents);
        }
    }

    let n = months.len();
    let get = |k: &str| g.get(k).cloned().unwrap_or_else(|| vec![0; n]);
    let sum = |a: &[i64], b: &[i64]| a.iter().zip(b).map(|(x, y)| x + y).collect::<Vec<_>>();
    let receita = get("receita_vendas");
    let deducoes = sum(&get("deducoes"), &get("impostos"));
    let liquida = sum(&receita, &deducoes);
    let custos = sum(&get("cmv"), &get("custos"));
    let bruto = sum(&liquida, &custos);
    let operacionais = get("despesas_operacionais");
    let pessoal = get("despesas_pessoal");
    let outras = get("outras_receitas");
    let financeiro = sum(&get("receitas_financeiras"), &get("despesas_financeiras"));
    let resultado = sum(
        &sum(&sum(&bruto, &operacionais), &sum(&pessoal, &outras)),
        &financeiro,
    );
    let investimentos = get("investimentos");
    let compras = get("compras_estoque");

    let line = |key: &str, label: &str, kind: &str, v: Vec<i64>| DreLine {
        total: from_cents(v.iter().sum()),
        values: v.into_iter().map(from_cents).collect(),
        key: key.into(),
        label: label.into(),
        kind: kind.into(),
    };
    let lines = vec![
        line(
            "receita_bruta",
            "Receita bruta (vendas e serviços)",
            "group",
            receita,
        ),
        line("deducoes", "(−) Deduções e impostos", "group", deducoes),
        line("receita_liquida", "= Receita líquida", "subtotal", liquida),
        line("custos", "(−) Custos (CMV e compras)", "group", custos),
        line("lucro_bruto", "= Lucro bruto", "subtotal", bruto),
        line(
            "despesas_operacionais",
            "(−) Despesas operacionais",
            "group",
            operacionais,
        ),
        line(
            "despesas_pessoal",
            "(−) Despesas com pessoal",
            "group",
            pessoal,
        ),
        line("outras_receitas", "(+) Outras receitas", "group", outras),
        line(
            "financeiro",
            "(±) Resultado financeiro",
            "group",
            financeiro,
        ),
        line("resultado", "= Resultado líquido", "subtotal", resultado),
        line(
            "compras_estoque",
            "Compras de mercadoria (viram custo na venda, pelo CMV)",
            "info",
            compras,
        ),
        line(
            "investimentos",
            "Investimentos (fora do resultado)",
            "info",
            investimentos,
        ),
    ];
    Ok(Dre { months, lines })
}
