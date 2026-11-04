//! Camada fina (equivale ao router): checa sessão/permissão e delega ao service.
//! Argumentos em snake_case também no JavaScript (`rename_all = "snake_case"`).

pub mod audit;
pub mod auth;
pub mod backup;
pub mod company;
pub mod users;

pub type Res<T> = Result<T, nucleo_core::CoreError>;
