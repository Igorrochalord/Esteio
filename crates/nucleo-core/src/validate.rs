//! Validações compartilhadas (CNPJ, e-mail, cor, tamanho de texto).

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidCnpj;

/// Normaliza o CNPJ para 14 dígitos. `Ok(None)` = vazio; `Err` = inválido.
pub fn cnpj(value: Option<&str>) -> Result<Option<String>, InvalidCnpj> {
    let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let digits: Vec<u32> = v.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 14 || digits.iter().all(|&d| d == digits[0]) {
        return Err(InvalidCnpj);
    }
    let check = |base: &[u32], weights: &[u32]| -> u32 {
        let rest = base.iter().zip(weights).map(|(d, w)| d * w).sum::<u32>() % 11;
        if rest < 2 {
            0
        } else {
            11 - rest
        }
    };
    let w1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let w2 = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let d1 = check(&digits[..12], &w1);
    let mut base = digits[..12].to_vec();
    base.push(d1);
    let d2 = check(&base, &w2);
    if digits[12] != d1 || digits[13] != d2 {
        return Err(InvalidCnpj);
    }
    Ok(Some(
        digits
            .iter()
            .map(|d| char::from_digit(*d, 10).unwrap_or('0'))
            .collect(),
    ))
}

/// Validação simples de formato; devolve o e-mail em minúsculas.
pub fn email(value: &str) -> Option<String> {
    let v = value.trim().to_lowercase();
    let (local, domain) = v.split_once('@')?;
    let ok = !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !v.contains(char::is_whitespace)
        && !domain.contains('@')
        && v.len() <= 254;
    ok.then_some(v)
}

pub fn hex_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Tamanho em caracteres (não bytes) após `trim`.
pub fn len_between(value: &str, min: usize, max: usize) -> bool {
    let n = value.trim().chars().count();
    (min..=max).contains(&n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cnpj_rules() {
        assert_eq!(
            cnpj(Some("11.222.333/0001-81")),
            Ok(Some("11222333000181".into()))
        );
        assert_eq!(cnpj(Some("  ")), Ok(None));
        assert_eq!(cnpj(None), Ok(None));
        assert!(cnpj(Some("11.222.333/0001-82")).is_err());
        assert!(cnpj(Some("11111111111111")).is_err());
    }

    #[test]
    fn email_rules() {
        assert_eq!(
            email(" Dono@Pao.com.br ").as_deref(),
            Some("dono@pao.com.br")
        );
        assert!(email("sem-arroba").is_none());
        assert!(email("a@b").is_none());
        assert!(email("a b@c.com").is_none());
    }

    #[test]
    fn color_rules() {
        assert!(hex_color("#2563eb"));
        assert!(!hex_color("2563eb"));
        assert!(!hex_color("#2563eg"));
    }
}
