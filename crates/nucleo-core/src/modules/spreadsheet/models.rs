use serde::{Deserialize, Serialize};

/// Uma aba a exportar: o que está na tela (já filtrado/ordenado) vai para o Excel.
#[derive(Debug, Deserialize)]
pub struct SheetInput {
    pub name: String,
    pub columns: Vec<ColumnInput>,
    /// linhas de valores (texto, número ou número em texto, ex.: "12.50")
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ColumnInput {
    pub label: String,
    /// text | number | money | percent | date | datetime
    #[serde(default = "text_kind")]
    pub kind: String,
}

fn text_kind() -> String {
    "text".into()
}

/// Campos do produto que a importação conhece.
pub const IMPORT_FIELDS: [&str; 9] = [
    "code",
    "name",
    "barcode",
    "category",
    "unit",
    "price",
    "cost",
    "stock",
    "min_stock",
];

#[derive(Debug, Serialize)]
pub struct ImportPreview {
    pub headers: Vec<String>,
    /// primeiras linhas, como texto
    pub sample: Vec<Vec<String>>,
    pub total_rows: usize,
    /// campo → índice da coluna sugerida (pelo nome do cabeçalho)
    pub suggested: std::collections::BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub path: String,
    /// campo → índice da coluna
    pub mapping: std::collections::BTreeMap<String, usize>,
    /// produto com o mesmo código: atualiza (true) ou ignora (false)
    #[serde(default)]
    pub update_existing: bool,
}

#[derive(Debug, Serialize)]
pub struct RowError {
    /// número da linha na planilha (1 = cabeçalho)
    pub row: usize,
    pub message: String,
}

#[derive(Debug, Default, Serialize)]
pub struct ImportResult {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<RowError>,
}
