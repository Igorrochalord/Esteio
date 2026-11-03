pub mod audit;
pub mod auth;
pub mod backup;
pub mod company;
pub mod users;

use serde::{Deserialize, Deserializer};

/// Distingue campo ausente (`None`) de `null` explícito (`Some(None)`) em updates parciais.
pub(crate) fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}
