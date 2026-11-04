//! Extrato bancário OFX (SGML 1.x ou XML 2.x): leitura, prévia de conciliação e importação.
//! Conciliar = achar na mesma conta um lançamento de mesmo valor, até 3 dias de diferença,
//! ainda sem extrato. O FITID do banco evita importar a mesma linha duas vezes.

use std::path::Path;

use chrono::NaiveDate;

use crate::error::{CoreError, CoreResult};
use crate::modules::spreadsheet::service::{decode, parse_number};
use crate::money::{from_cents, to_cents};
use crate::Ctx;

use super::models::{OfxImportRequest, OfxImportResult, OfxLine, OfxPreview};
use super::service::{ensure_default_categories, insert_tx, NewTx};

pub struct OfxTx {
    pub fitid: String,
    pub date: NaiveDate,
    pub amount_cents: i64,
    pub memo: String,
}

pub struct OfxDoc {
    pub bank_id: Option<String>,
    pub acct_id: Option<String>,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    pub ledger_cents: Option<i64>,
    pub txs: Vec<OfxTx>,
}

/// Valor da tag no trecho: `<TAG>valor` até o próximo `<` ou fim de linha (SGML e XML).
fn tag(block: &str, name: &str) -> Option<String> {
    let upper = block.to_uppercase();
    let open = format!("<{name}>");
    let start = upper.find(&open)? + open.len();
    let rest = &block[start..];
    let end = rest.find(['<', '\n', '\r']).unwrap_or(rest.len());
    let v = rest[..end].trim();
    (!v.is_empty()).then(|| v.to_owned())
}

/// "20261103120000[-3:BRT]" → 2026-11-03
fn ofx_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.get(..8)?, "%Y%m%d").ok()
}

pub fn parse(text: &str) -> CoreResult<OfxDoc> {
    let invalid = || CoreError::BusinessRule("Arquivo OFX inválido ou vazio".into());
    let upper = text.to_uppercase();
    if !upper.contains("<OFX>") {
        return Err(invalid());
    }
    let mut txs = Vec::new();
    let mut pos = 0;
    while let Some(i) = upper[pos..].find("<STMTTRN>") {
        let begin = pos + i + "<STMTTRN>".len();
        let end = upper[begin..]
            .find("</STMTTRN>")
            .or_else(|| upper[begin..].find("<STMTTRN>"))
            .map(|e| begin + e)
            .unwrap_or(text.len());
        let block = &text[begin..end];
        pos = end;
        let Some(date) = tag(block, "DTPOSTED").and_then(|d| ofx_date(&d)) else {
            continue;
        };
        let Some(amount) = tag(block, "TRNAMT")
            .and_then(|a| parse_number(&a))
            .and_then(to_cents)
        else {
            continue;
        };
        if amount == 0 {
            continue;
        }
        let memo = tag(block, "MEMO")
            .or_else(|| tag(block, "NAME"))
            .unwrap_or_else(|| "Lançamento do extrato".into());
        let fitid = tag(block, "FITID")
            .unwrap_or_else(|| format!("{}{amount}{memo}", date.format("%Y%m%d")));
        txs.push(OfxTx {
            fitid,
            date,
            amount_cents: amount,
            memo,
        });
    }
    if txs.is_empty() {
        return Err(CoreError::BusinessRule(
            "O extrato não tem lançamentos".into(),
        ));
    }
    let ledger = upper
        .find("<LEDGERBAL>")
        .and_then(|i| tag(&text[i..], "BALAMT"))
        .and_then(|b| parse_number(&b))
        .and_then(to_cents);
    Ok(OfxDoc {
        bank_id: tag(text, "BANKID"),
        acct_id: tag(text, "ACCTID"),
        start: tag(text, "DTSTART").and_then(|d| ofx_date(&d)),
        end: tag(text, "DTEND").and_then(|d| ofx_date(&d)),
        ledger_cents: ledger,
        txs,
    })
}

fn read(path: &str) -> CoreResult<OfxDoc> {
    let bytes = std::fs::read(Path::new(path))
        .map_err(|_| CoreError::BusinessRule("Não foi possível ler o arquivo".into()))?;
    parse(&decode(&bytes))
}

/// Situação de cada linha: (status, id do lançamento correspondente, descrição).
async fn classify(
    conn: &mut crate::db::Conn,
    account: &str,
    doc: &OfxDoc,
) -> CoreResult<Vec<(String, Option<String>, Option<String>)>> {
    let mut used: Vec<String> = Vec::new();
    let mut out = Vec::with_capacity(doc.txs.len());
    for t in &doc.txs {
        let dup: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM bank_transactions WHERE bank_account_id = ? AND ofx_fitid = ? AND deleted_at IS NULL",
        )
        .bind(account)
        .bind(&t.fitid)
        .fetch_one(&mut *conn)
        .await?;
        if dup > 0 {
            out.push(("ja_importado".to_owned(), None, None));
            continue;
        }
        let from = (t.date - chrono::Duration::days(3))
            .format("%Y-%m-%d")
            .to_string();
        let to = (t.date + chrono::Duration::days(3))
            .format("%Y-%m-%d")
            .to_string();
        let candidates: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, description, date FROM bank_transactions
             WHERE bank_account_id = ? AND amount_cents = ? AND ofx_fitid IS NULL AND deleted_at IS NULL
               AND date BETWEEN ? AND ?
             ORDER BY abs(julianday(date) - julianday(?)), created_at",
        )
        .bind(account)
        .bind(t.amount_cents)
        .bind(&from)
        .bind(&to)
        .bind(t.date.format("%Y-%m-%d").to_string())
        .fetch_all(&mut *conn)
        .await?;
        match candidates.into_iter().find(|c| !used.contains(&c.0)) {
            Some((id, desc, _)) => {
                used.push(id.clone());
                out.push(("conciliar".to_owned(), Some(id), Some(desc)));
            }
            None => out.push(("novo".to_owned(), None, None)),
        }
    }
    Ok(out)
}

// PLANO: conciliação bancária é recurso do Starter em diante (hoje liberado).
pub async fn preview(ctx: &Ctx, account_id: &str, path: &str) -> CoreResult<OfxPreview> {
    let doc = read(path)?;
    let mut conn = ctx.db.acquire().await?;
    let status = classify(&mut conn, account_id, &doc).await?;
    let fmt = |d: Option<NaiveDate>| d.map(|d| d.format("%Y-%m-%d").to_string());
    Ok(OfxPreview {
        bank_id: doc.bank_id.clone(),
        account_id: doc.acct_id.clone(),
        start: fmt(doc.start),
        end: fmt(doc.end),
        ledger_balance: doc.ledger_cents.map(from_cents),
        lines: doc
            .txs
            .iter()
            .zip(status)
            .map(|(t, (st, id, desc))| OfxLine {
                fitid: t.fitid.clone(),
                date: t.date.format("%Y-%m-%d").to_string(),
                amount: from_cents(t.amount_cents),
                memo: t.memo.clone(),
                status: st,
                match_id: id,
                match_description: desc,
            })
            .collect(),
    })
}

pub async fn import(ctx: &Ctx, req: OfxImportRequest) -> CoreResult<OfxImportResult> {
    let doc = read(&req.path)?;
    let mut tx = ctx.db.begin().await?;
    let ok: Option<bool> =
        sqlx::query_scalar("SELECT active FROM bank_accounts WHERE id = ? AND deleted_at IS NULL")
            .bind(&req.bank_account_id)
            .fetch_optional(&mut *tx)
            .await?;
    if ok != Some(true) {
        return Err(CoreError::field(
            "bank_account_id",
            "Conta inválida ou inativa",
        ));
    }
    ensure_default_categories(&mut tx, &ctx.inst).await?;
    let status = classify(&mut tx, &req.bank_account_id, &doc).await?;
    let mut result = OfxImportResult::default();
    for (t, (st, id, _)) in doc.txs.iter().zip(status) {
        match st.as_str() {
            "conciliar" => {
                sqlx::query("UPDATE bank_transactions SET reconciled = 1, ofx_fitid = ?, updated_at = ? WHERE id = ?")
                    .bind(&t.fitid)
                    .bind(crate::db::now())
                    .bind(id.as_deref())
                    .execute(&mut *tx)
                    .await?;
                result.reconciled += 1;
            }
            "novo" if req.create_new => {
                let category = if t.amount_cents > 0 {
                    req.income_category_id.as_deref()
                } else {
                    req.expense_category_id.as_deref()
                };
                let date = t.date.format("%Y-%m-%d").to_string();
                insert_tx(
                    &mut tx,
                    &ctx.inst,
                    NewTx {
                        account_id: &req.bank_account_id,
                        date: &date,
                        amount_cents: t.amount_cents,
                        description: &t.memo,
                        category_id: category,
                        entry_id: None,
                        sale_id: None,
                        transfer_group: None,
                        fitid: Some(&t.fitid),
                        reconciled: true,
                    },
                )
                .await?;
                result.created += 1;
            }
            _ => result.skipped += 1,
        }
    }
    let file = Path::new(&req.path)
        .file_name()
        .map(|f| f.to_string_lossy().into_owned());
    let e = crate::audit::Entry {
        actor_id: Some(&ctx.user.id),
        action: "bank_tx.ofx_import",
        entity: "bank_account",
        entity_id: Some(&req.bank_account_id),
        before: None,
        after: Some(
            serde_json::json!({"file": file, "reconciled": result.reconciled, "created": result.created, "skipped": result.skipped}),
        ),
    };
    crate::audit::record(&mut tx, &ctx.inst, e).await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sgml_with_brazilian_amounts() {
        let ofx = "OFXHEADER:100\nDATA:OFXSGML\n<OFX><BANKMSGSRSV1><STMTTRNRS><STMTRS>\n<BANKACCTFROM><BANKID>341<ACCTID>12345-6</BANKACCTFROM>\n\
            <BANKTRANLIST><DTSTART>20261001<DTEND>20261031\n\
            <STMTTRN><TRNTYPE>CREDIT<DTPOSTED>20261003120000[-3:BRT]<TRNAMT>150,00<FITID>A1<MEMO>PIX RECEBIDO</STMTTRN>\n\
            <STMTTRN><TRNTYPE>DEBIT<DTPOSTED>20261005<TRNAMT>-89.90<FITID>A2<NAME>ENERGIA\n\
            </BANKTRANLIST><LEDGERBAL><BALAMT>1060.10<DTASOF>20261031</LEDGERBAL></STMTRS></STMTTRNRS></BANKMSGSRSV1></OFX>";
        let doc = parse(ofx).unwrap();
        assert_eq!(doc.bank_id.as_deref(), Some("341"));
        assert_eq!(doc.txs.len(), 2);
        assert_eq!(
            (doc.txs[0].amount_cents, doc.txs[0].memo.as_str()),
            (15000, "PIX RECEBIDO")
        );
        assert_eq!(
            (doc.txs[1].amount_cents, doc.txs[1].fitid.as_str()),
            (-8990, "A2")
        );
        assert_eq!(doc.txs[1].memo, "ENERGIA");
        assert_eq!(doc.ledger_cents, Some(106010));
        assert!(parse("não é ofx").is_err());
    }
}
