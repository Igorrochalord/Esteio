//! Dinheiro e quantidades: `Decimal` no domínio e na API (JSON como texto, ex. "12.50"),
//! inteiros no banco (centavos e milésimos) para somas exatas no SQLite. Nunca float.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};

/// Centavos → reais.
pub fn from_cents(cents: i64) -> Decimal {
    Decimal::new(cents, 2)
}

/// Reais → centavos (arredonda meio centavo para cima). `None` se não couber.
pub fn to_cents(value: Decimal) -> Option<i64> {
    (value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero) * Decimal::ONE_HUNDRED)
        .to_i64()
}

/// Milésimos → quantidade (1000 = 1 un; 1250 = 1,25 kg).
pub fn from_milli(milli: i64) -> Decimal {
    Decimal::new(milli, 3).normalize()
}

/// Quantidade → milésimos (até 3 casas). `None` se não couber.
pub fn to_milli(value: Decimal) -> Option<i64> {
    (value.round_dp_with_strategy(3, RoundingStrategy::MidpointAwayFromZero)
        * Decimal::ONE_THOUSAND)
        .to_i64()
}

/// Valor de `qty_milli` unidades a `unit_cents` cada, em centavos (arredondado).
pub fn line_cents(qty_milli: i64, unit_cents: i64) -> i64 {
    // (qty/1000) * unit, com arredondamento meio-para-cima, em inteiros
    let product = i128::from(qty_milli) * i128::from(unit_cents);
    let rounded = (product + if product >= 0 { 500 } else { -500 }) / 1000;
    rounded as i64
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(from_cents(1250).to_string(), "12.50");
        assert_eq!(to_cents(Decimal::from_str("12.345").unwrap()), Some(1235));
        assert_eq!(from_milli(1250).to_string(), "1.25");
        assert_eq!(from_milli(3000).to_string(), "3");
        assert_eq!(to_milli(Decimal::from_str("0.0005").unwrap()), Some(1));
    }

    #[test]
    fn line_totals_round_half_up() {
        assert_eq!(line_cents(3000, 80), 240); // 3 × 0,80
        assert_eq!(line_cents(1250, 3990), 4988); // 1,25 kg × 39,90 = 49,875
        assert_eq!(line_cents(333, 100), 33);
    }
}
