//! Integrações online (ADR 0003). O app funciona sem internet; o que depende de API
//! (PIX, NFS-e, marketplaces) passa por aqui:
//!
//! - `provider`: contrato que cada provedor implementa (testar conexão, rodar trabalhos);
//! - `catalog`: provedores conhecidos, inclusive os ainda não implementados;
//! - `http`: cliente HTTP compartilhado (rustls, tempos limite, erros em pt-BR);
//! - `service`: configuração, credenciais cifradas (`secrets::Vault`) e a fila de trabalhos.

pub mod catalog;
pub mod http;
pub mod models;
pub mod provider;
pub mod providers;
pub mod repository;
pub mod service;
