//! Provedores embutidos no programa. Cada um implementa `provider::Provider`.

pub mod simulador;

use std::sync::Arc;

use crate::Core;

/// Registra os provedores que vêm com o programa (chamado em `Core::open`).
pub fn register_builtin(core: &Core) {
    core.register_provider(Arc::new(simulador::Simulador));
}
