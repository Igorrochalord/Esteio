//! PIX na venda (fase 6b): cobrança no provedor escolhido, QR gerado localmente, título a
//! receber até o pagamento cair e baixa automática quando a consulta confirma.
//!
//! Contrato dos trabalhos que todo provedor com `capabilities: ["pix"]` entende:
//! - `pix.create` `{charge_id, txid, amount_cents, description, expires_secs, payer?}` →
//!   `Done {provider_ref, copy_paste, expires_at}`;
//! - `pix.poll` `{charge_id, provider_ref, txid, created_at, expires_at}` →
//!   `Wait` enquanto não pago, `Done {status: "paid"|"expired"|"canceled", paid_at?}`;
//! - `pix.cancel` `{charge_id, provider_ref, txid}` → `Done {status: "canceled"}`.

pub mod models;
pub mod service;
