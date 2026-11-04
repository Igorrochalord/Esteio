//! Planilhas: exportar o que está na tela para .xlsx e importar produtos de
//! .xlsx/.xls/.ods/.csv (com prévia e mapeamento de colunas).

use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

use calamine::{open_workbook_auto, Data, Reader};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_xlsxwriter::{ExcelDateTime, Format, FormatAlign, Workbook};

use crate::audit::{self, Entry};
use crate::db;
use crate::error::{CoreError, CoreResult};
use crate::modules::products::models::{ProductRow, UNITS};
use crate::modules::products::repository::{self as products_repo, NewMovement};
use crate::modules::products::service::upsert_category;
use crate::money;
use crate::Ctx;

use super::models::*;

// ------------------------------------------------------------------ exportação

fn sheet_name(name: &str, index: usize) -> String {
    let clean: String = name
        .chars()
        .filter(|c| !matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\'))
        .take(31)
        .collect();
    if clean.trim().is_empty() {
        format!("Planilha{}", index + 1)
    } else {
        clean
    }
}

fn as_decimal(v: &serde_json::Value) -> Option<Decimal> {
    match v {
        serde_json::Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        serde_json::Value::String(s) => parse_number(s),
        serde_json::Value::Bool(b) => Some(Decimal::from(i32::from(*b))),
        _ => None,
    }
}

fn as_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => if *b { "sim" } else { "não" }.into(),
        other => other.to_string(),
    }
}

fn xlsx_err(e: rust_xlsxwriter::XlsxError) -> CoreError {
    CoreError::Internal(format!("xlsx: {e}"))
}

/// Grava as abas num arquivo .xlsx (sobrescreve: o "Salvar como" já confirmou).
pub fn export_xlsx(path: &Path, sheets: &[SheetInput]) -> CoreResult<()> {
    if sheets.is_empty() {
        return Err(CoreError::BusinessRule("Nada para exportar".into()));
    }
    let mut wb = Workbook::new();
    let header = Format::new()
        .set_bold()
        .set_background_color(0x0F172A)
        .set_font_color(0xFFFFFF)
        .set_align(FormatAlign::Center);
    let money_fmt = Format::new().set_num_format("\"R$\" #,##0.00");
    let number_fmt = Format::new().set_num_format("#,##0.###");
    let percent_fmt = Format::new().set_num_format("0.0%");
    let date_fmt = Format::new().set_num_format("dd/mm/yyyy");
    let datetime_fmt = Format::new().set_num_format("dd/mm/yyyy hh:mm");

    for (si, sheet) in sheets.iter().enumerate() {
        let ws = wb.add_worksheet();
        ws.set_name(sheet_name(&sheet.name, si)).map_err(xlsx_err)?;
        for (c, col) in sheet.columns.iter().enumerate() {
            ws.write_with_format(0, c as u16, &col.label, &header)
                .map_err(xlsx_err)?;
        }
        for (r, row) in sheet.rows.iter().enumerate() {
            let r = (r + 1) as u32;
            for (c, col) in sheet.columns.iter().enumerate() {
                let c16 = c as u16;
                let Some(value) = row.get(c).filter(|v| !v.is_null()) else {
                    continue;
                };
                let written = match col.kind.as_str() {
                    "money" => write_numeric(ws, r, c16, value, &money_fmt, Decimal::ONE)?,
                    "number" => write_numeric(ws, r, c16, value, &number_fmt, Decimal::ONE)?,
                    "percent" => {
                        write_numeric(ws, r, c16, value, &percent_fmt, Decimal::ONE_HUNDRED)?
                    }
                    "date" | "datetime" => {
                        let text = as_text(value);
                        let iso = text.trim_end_matches('Z');
                        let iso = iso.split('.').next().unwrap_or(iso);
                        match ExcelDateTime::parse_from_str(iso) {
                            Ok(dt) => {
                                let fmt = if col.kind == "date" {
                                    &date_fmt
                                } else {
                                    &datetime_fmt
                                };
                                ws.write_datetime_with_format(r, c16, &dt, fmt)
                                    .map_err(xlsx_err)?;
                                true
                            }
                            Err(_) => false,
                        }
                    }
                    _ => false,
                };
                if !written {
                    ws.write(r, c16, as_text(value)).map_err(xlsx_err)?;
                }
            }
        }
        let last_col = sheet.columns.len().saturating_sub(1) as u16;
        ws.set_freeze_panes(1, 0).map_err(xlsx_err)?;
        if !sheet.columns.is_empty() {
            ws.autofilter(0, 0, sheet.rows.len() as u32, last_col)
                .map_err(xlsx_err)?;
        }
        ws.autofit();
    }
    wb.save(path).map_err(xlsx_err)?;
    Ok(())
}

/// Escreve como número (o .xlsx guarda double). `scale` divide antes (percentual: 100).
/// Devolve `false` se o valor não for numérico, para cair no texto.
fn write_numeric(
    ws: &mut rust_xlsxwriter::Worksheet,
    r: u32,
    c: u16,
    value: &serde_json::Value,
    fmt: &Format,
    scale: Decimal,
) -> CoreResult<bool> {
    match as_decimal(value).and_then(|d| (d / scale).to_f64()) {
        Some(n) => {
            ws.write_number_with_format(r, c, n, fmt)
                .map_err(xlsx_err)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

// ------------------------------------------------------------------ leitura

/// Número em formato brasileiro ou internacional: "R$ 1.234,56", "1234.56", "12,5".
pub fn parse_number(raw: &str) -> Option<Decimal> {
    let s: String = raw
        .trim()
        .trim_start_matches("R$")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}')
        .collect();
    if s.is_empty() {
        return None;
    }
    let normalized = match (s.rfind(','), s.rfind('.')) {
        (Some(comma), Some(dot)) if comma > dot => s.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => s.replace(',', ""),
        (Some(_), None) => s.replace(',', "."),
        _ => s,
    };
    Decimal::from_str(&normalized).ok()
}

fn cell_text(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_owned(),
        Data::Float(f) => {
            // 32.0 → "32"; 0.8 → "0.8" (sem notação científica para valores comuns)
            let s = format!("{f}");
            s.strip_suffix(".0").map(str::to_owned).unwrap_or(s)
        }
        other => other.to_string().trim().to_owned(),
    }
}

/// Bytes → texto: UTF-8 (com ou sem BOM) ou, se falhar, Windows-1252/Latin-1 (CSV do Excel).
pub(crate) fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// Primeira aba (ou o CSV) como linhas de texto; a primeira linha é o cabeçalho.
fn read_table(path: &Path) -> CoreResult<Vec<Vec<String>>> {
    let unreadable = || {
        CoreError::BusinessRule(
            "Não foi possível ler o arquivo. Use .xlsx, .xls, .ods ou .csv".into(),
        )
    };
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = if ext == "csv" || ext == "txt" {
        let text = decode(&std::fs::read(path).map_err(|_| unreadable())?);
        let first = text.lines().next().unwrap_or_default();
        let delimiter = if first.matches(';').count() >= first.matches(',').count() {
            b';'
        } else {
            b','
        };
        csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(false)
            .flexible(true)
            .from_reader(text.as_bytes())
            .records()
            .map(|r| {
                r.map(|rec| rec.iter().map(|c| c.trim().to_owned()).collect())
                    .map_err(|_| unreadable())
            })
            .collect::<CoreResult<_>>()?
    } else {
        let mut wb = open_workbook_auto(path).map_err(|_| unreadable())?;
        let range = wb
            .worksheet_range_at(0)
            .ok_or_else(unreadable)?
            .map_err(|_| unreadable())?;
        range
            .rows()
            .map(|r| r.iter().map(cell_text).collect())
            .collect()
    };
    let rows: Vec<Vec<String>> = rows
        .into_iter()
        .skip_while(|r| r.iter().all(String::is_empty))
        .collect();
    if rows.is_empty() {
        return Err(CoreError::BusinessRule("A planilha está vazia".into()));
    }
    Ok(rows)
}

/// Minúsculas e sem acento, para comparar cabeçalhos.
fn normalize(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'ê' | 'è' => 'e',
            'í' | 'î' => 'i',
            'ó' | 'ô' | 'õ' | 'ò' => 'o',
            'ú' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            '_' | '-' | '.' => ' ',
            other => other,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ordem importa: "código de barras" antes de "código", "estoque mínimo" antes de "estoque",
/// "preço de custo" antes de "preço".
const ALIASES: [(&str, &[&str]); 9] = [
    (
        "barcode",
        &["codigo de barras", "cod barras", "barras", "ean", "gtin"],
    ),
    (
        "min_stock",
        &["estoque minimo", "qtd minima", "minimo", "min"],
    ),
    (
        "cost",
        &["preco de custo", "valor de custo", "custo", "compra"],
    ),
    ("code", &["codigo", "cod", "sku", "referencia", "ref"]),
    ("name", &["nome", "produto", "descricao", "item"]),
    ("category", &["categoria", "grupo", "departamento", "secao"]),
    ("unit", &["unidade", "un", "und", "medida"]),
    (
        "price",
        &[
            "preco de venda",
            "preco venda",
            "valor de venda",
            "preco",
            "venda",
            "valor",
        ],
    ),
    (
        "stock",
        &["estoque atual", "estoque", "quantidade", "qtd", "saldo"],
    ),
];

fn suggest(headers: &[String]) -> BTreeMap<String, usize> {
    let norm: Vec<String> = headers.iter().map(|h| normalize(h)).collect();
    let mut taken = vec![false; headers.len()];
    let mut out = BTreeMap::new();
    for exact in [true, false] {
        for (field, aliases) in ALIASES {
            if out.contains_key(field) {
                continue;
            }
            let found = norm.iter().enumerate().find(|(i, h)| {
                !taken[*i]
                    && aliases.iter().any(|a| {
                        if exact {
                            *h == a
                        } else {
                            h.split(' ').any(|w| w == *a) || h.starts_with(a)
                        }
                    })
            });
            if let Some((i, _)) = found {
                taken[i] = true;
                out.insert(field.to_owned(), i);
            }
        }
    }
    out
}

pub fn preview(path: &Path) -> CoreResult<ImportPreview> {
    let rows = read_table(path)?;
    let headers = rows[0].clone();
    let suggested = suggest(&headers);
    Ok(ImportPreview {
        total_rows: rows.len() - 1,
        sample: rows.iter().skip(1).take(20).cloned().collect(),
        suggested,
        headers,
    })
}

// ------------------------------------------------------------------ importação

struct Parsed {
    code: Option<String>,
    name: Option<String>,
    barcode: Option<String>,
    category: Option<String>,
    unit: Option<String>,
    price: Option<i64>,
    cost: Option<i64>,
    stock: Option<i64>,
    min_stock: Option<i64>,
}

fn parse_row(row: &[String], mapping: &BTreeMap<String, usize>) -> Result<Parsed, String> {
    let get = |field: &str| -> Option<String> {
        mapping
            .get(field)
            .and_then(|&i| row.get(i))
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let money_of = |field: &str, label: &str| -> Result<Option<i64>, String> {
        match get(field) {
            None => Ok(None),
            Some(raw) => parse_number(&raw)
                .and_then(money::to_cents)
                .filter(|c| *c >= 0)
                .map(Some)
                .ok_or_else(|| format!("{label} inválido: \"{raw}\"")),
        }
    };
    let qty_of = |field: &str, label: &str| -> Result<Option<i64>, String> {
        match get(field) {
            None => Ok(None),
            Some(raw) => parse_number(&raw)
                .and_then(money::to_milli)
                .filter(|q| *q >= 0)
                .map(Some)
                .ok_or_else(|| format!("{label} inválido: \"{raw}\"")),
        }
    };
    let unit = match get("unit") {
        None => None,
        Some(u) => {
            let u = u.to_lowercase();
            let u = match u.as_str() {
                "und" | "unid" | "unidade" | "pc" | "pç" => "un".to_owned(),
                "lt" | "litro" => "l".to_owned(),
                "quilo" | "kilo" => "kg".to_owned(),
                _ => u,
            };
            if !UNITS.contains(&u.as_str()) {
                return Err(format!("Unidade desconhecida: \"{u}\""));
            }
            Some(u)
        }
    };
    let name = get("name");
    if name.as_ref().is_some_and(|n| n.chars().count() > 200) {
        return Err("Nome com mais de 200 caracteres".into());
    }
    let code = get("code");
    if code.as_ref().is_some_and(|c| c.chars().count() > 40) {
        return Err("Código com mais de 40 caracteres".into());
    }
    Ok(Parsed {
        code,
        name,
        barcode: get("barcode"),
        category: get("category"),
        unit,
        price: money_of("price", "Preço")?,
        cost: money_of("cost", "Custo")?,
        stock: qty_of("stock", "Estoque")?,
        min_stock: qty_of("min_stock", "Estoque mínimo")?,
    })
}

/// Importa produtos numa única transação. Linhas com problema são puladas e relatadas;
/// as demais entram. Produto com o mesmo código é atualizado (se pedido) ou ignorado.
pub async fn import_products(ctx: &Ctx, req: ImportRequest) -> CoreResult<ImportResult> {
    if !req.mapping.contains_key("name") {
        return Err(CoreError::field(
            "mapping",
            "Escolha a coluna do nome do produto",
        ));
    }
    if let Some(bad) = req
        .mapping
        .keys()
        .find(|k| !IMPORT_FIELDS.contains(&k.as_str()))
    {
        return Err(CoreError::field(
            "mapping",
            &format!("Campo desconhecido: {bad}"),
        ));
    }
    let rows = read_table(Path::new(&req.path))?;
    let mut result = ImportResult::default();
    let mut tx = ctx.db.begin().await?;

    for (i, row) in rows.iter().enumerate().skip(1) {
        let line = i + 1; // linha como o usuário vê na planilha
        if row.iter().all(String::is_empty) {
            continue;
        }
        let p = match parse_row(row, &req.mapping) {
            Ok(p) => p,
            Err(message) => {
                result.errors.push(RowError { row: line, message });
                continue;
            }
        };
        let category_id = match &p.category {
            Some(name) => Some(upsert_category(&mut tx, ctx, name).await?.id),
            None => None,
        };
        let existing = match &p.code {
            Some(code) => products_repo::find_by_code(&mut tx, code).await?,
            None => None,
        };
        let now = db::now();

        if let Some(mut prod) = existing {
            if !req.update_existing {
                result.skipped += 1;
                continue;
            }
            if let Some(v) = p.name {
                prod.name = v;
            }
            if p.barcode.is_some() {
                prod.barcode = p.barcode;
            }
            if category_id.is_some() {
                prod.category_id = category_id;
            }
            if let Some(v) = p.unit {
                prod.unit = v;
            }
            if let Some(v) = p.price {
                prod.price_cents = v;
            }
            if let Some(v) = p.cost {
                prod.cost_cents = v;
            }
            if let Some(v) = p.min_stock {
                prod.min_stock_milli = v;
            }
            prod.updated_at = now;
            products_repo::update(&mut tx, &prod).await?;
            if let (Some(count), true) = (p.stock, prod.track_stock) {
                let delta = count - prod.stock_milli;
                if delta != 0 {
                    let m = NewMovement {
                        product_id: &prod.id,
                        kind: "ajuste",
                        delta_milli: delta,
                        unit_cost_cents: None,
                        note: Some("Importação de planilha"),
                        sale_id: None,
                        user_id: Some(&ctx.user.id),
                    };
                    products_repo::apply_movement(&mut tx, &ctx.inst, m).await?;
                }
            }
            result.updated += 1;
            continue;
        }

        let Some(name) = p.name else {
            result.errors.push(RowError {
                row: line,
                message: "Nome do produto vazio".into(),
            });
            continue;
        };
        let code = match p.code {
            Some(c) => c,
            None => products_repo::next_code(&mut tx).await?,
        };
        let row = ProductRow {
            id: db::new_id(),
            code,
            barcode: p.barcode,
            name,
            category_id,
            category_name: None,
            unit: p.unit.unwrap_or_else(|| "un".into()),
            price_cents: p.price.unwrap_or(0),
            cost_cents: p.cost.unwrap_or(0),
            track_stock: true,
            stock_milli: 0,
            min_stock_milli: p.min_stock.unwrap_or(0),
            active: true,
            created_at: now.clone(),
            updated_at: now,
        };
        products_repo::insert(&mut tx, &ctx.inst, &row).await?;
        if let Some(qty) = p.stock.filter(|q| *q > 0) {
            let m = NewMovement {
                product_id: &row.id,
                kind: "entrada",
                delta_milli: qty,
                unit_cost_cents: Some(row.cost_cents),
                note: Some("Importação de planilha"),
                sale_id: None,
                user_id: Some(&ctx.user.id),
            };
            products_repo::apply_movement(&mut tx, &ctx.inst, m).await?;
        }
        result.created += 1;
    }

    let file = Path::new(&req.path)
        .file_name()
        .map(|f| f.to_string_lossy().into_owned());
    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action: "product.import",
        entity: "product",
        entity_id: None,
        before: None,
        after: Some(serde_json::json!({
            "file": file,
            "created": result.created,
            "updated": result.updated,
            "skipped": result.skipped,
            "errors": result.errors.len(),
        })),
    };
    audit::record(&mut tx, &ctx.inst, entry).await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_in_any_format() {
        let n = |s: &str| parse_number(s).map(|d| d.normalize().to_string());
        assert_eq!(n("R$ 1.234,56").as_deref(), Some("1234.56"));
        assert_eq!(n("1,234.56").as_deref(), Some("1234.56"));
        assert_eq!(n("12,5").as_deref(), Some("12.5"));
        assert_eq!(n("0.80").as_deref(), Some("0.8"));
        assert_eq!(n("  ").as_deref(), None);
        assert_eq!(n("abc").as_deref(), None);
    }

    #[test]
    fn header_suggestions() {
        let h: Vec<String> = [
            "Código",
            "Descrição",
            "Cód. Barras",
            "Preço de Custo",
            "Preço Venda",
            "Estoque Mínimo",
            "Estoque",
            "Grupo",
            "UN",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let s = suggest(&h);
        assert_eq!(s["code"], 0);
        assert_eq!(s["name"], 1);
        assert_eq!(s["barcode"], 2);
        assert_eq!(s["cost"], 3);
        assert_eq!(s["price"], 4);
        assert_eq!(s["min_stock"], 5);
        assert_eq!(s["stock"], 6);
        assert_eq!(s["category"], 7);
        assert_eq!(s["unit"], 8);
    }
}
