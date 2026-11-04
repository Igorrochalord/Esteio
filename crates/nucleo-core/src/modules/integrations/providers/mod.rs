//! Provedores embutidos no programa. Cada um implementa `provider::Provider`.

pub mod asaas;
pub mod efi;
pub mod mercadopago;
pub mod pix_common;
pub mod simulador;

use std::sync::Arc;

use crate::Core;

/// Registra os provedores que vêm com o programa (chamado em `Core::open`).
pub fn register_builtin(core: &Core) {
    core.register_provider(Arc::new(simulador::Simulador));
    core.register_provider(Arc::new(mercadopago::MercadoPago));
    core.register_provider(Arc::new(asaas::Asaas));
    core.register_provider(Arc::new(efi::Efi));
}
