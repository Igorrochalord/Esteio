use nucleo_core::modules::finance::models::*;
use nucleo_core::modules::finance::reports::{self, CashFlow, CashFlowQuery, Dre, DreQuery};
use nucleo_core::modules::finance::{ofx, service as fin};
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, CoreResult, Ctx};
use tauri::State;

use super::Res;

async fn read(core: &Core) -> CoreResult<Ctx> {
    core.require(Module::Finance, Action::Read).await
}
async fn write(core: &Core) -> CoreResult<Ctx> {
    core.require(Module::Finance, Action::Write).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_accounts(core: State<'_, Core>) -> Res<Vec<Account>> {
    fin::accounts(&read(&core).await?).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_account_create(core: State<'_, Core>, data: AccountInput) -> Res<Account> {
    fin::create_account(&write(&core).await?, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_account_update(
    core: State<'_, Core>,
    id: String,
    data: AccountPatch,
) -> Res<Account> {
    fin::update_account(&write(&core).await?, &id, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_categories(core: State<'_, Core>) -> Res<Vec<Category>> {
    fin::categories(&read(&core).await?).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_category_create(core: State<'_, Core>, data: CategoryInput) -> Res<Category> {
    fin::create_category(&write(&core).await?, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_category_update(
    core: State<'_, Core>,
    id: String,
    data: CategoryPatch,
) -> Res<()> {
    fin::update_category(&write(&core).await?, &id, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_entries(core: State<'_, Core>, query: EntryQuery) -> Res<Vec<Entry>> {
    fin::entries(&read(&core).await?, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_entry_create(core: State<'_, Core>, data: EntryInput) -> Res<Vec<Entry>> {
    fin::create_entry(&write(&core).await?, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_entry_update(core: State<'_, Core>, id: String, data: EntryPatch) -> Res<Entry> {
    fin::update_entry(&write(&core).await?, &id, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_entry_pay(core: State<'_, Core>, id: String, data: PayInput) -> Res<Entry> {
    fin::pay_entry(&write(&core).await?, &id, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_entry_unpay(core: State<'_, Core>, id: String) -> Res<Entry> {
    fin::unpay_entry(&write(&core).await?, &id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_entry_cancel(core: State<'_, Core>, id: String) -> Res<Entry> {
    fin::cancel_entry(&write(&core).await?, &id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_transactions(core: State<'_, Core>, query: TxQuery) -> Res<Vec<Transaction>> {
    fin::transactions(&read(&core).await?, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_transaction_create(core: State<'_, Core>, data: TxInput) -> Res<()> {
    fin::create_transaction(&write(&core).await?, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_transfer(core: State<'_, Core>, data: TransferInput) -> Res<()> {
    fin::transfer(&write(&core).await?, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_transaction_delete(core: State<'_, Core>, id: String) -> Res<()> {
    fin::delete_transaction(&write(&core).await?, &id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_ofx_preview(
    core: State<'_, Core>,
    bank_account_id: String,
    path: String,
) -> Res<OfxPreview> {
    ofx::preview(&write(&core).await?, &bank_account_id, &path).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_ofx_import(core: State<'_, Core>, data: OfxImportRequest) -> Res<OfxImportResult> {
    ofx::import(&write(&core).await?, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_cash_flow(core: State<'_, Core>, query: CashFlowQuery) -> Res<CashFlow> {
    reports::cash_flow(&read(&core).await?, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_dre(core: State<'_, Core>, query: DreQuery) -> Res<Dre> {
    reports::dre(&read(&core).await?, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fin_summary(core: State<'_, Core>, today: String) -> Res<FinanceSummary> {
    fin::summary(&read(&core).await?, &today).await
}
