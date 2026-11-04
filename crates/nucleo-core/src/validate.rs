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

/// Normaliza CPF (11 dígitos) ou CNPJ (14 dígitos) conforme o tamanho. `Ok(None)` = vazio.
pub fn document(value: Option<&str>) -> Result<Option<String>, InvalidCnpj> {
    let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let digits: Vec<u32> = v.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() == 14 {
        return cnpj(Some(v));
    }
    if digits.len() != 11 || digits.iter().all(|&d| d == digits[0]) {
        return Err(InvalidCnpj);
    }
    let check = |len: usize| -> u32 {
        let sum: u32 = digits[..len]
            .iter()
            .enumerate()
            .map(|(i, d)| d * (len as u32 + 1 - i as u32))
            .sum();
        let rest = (sum * 10) % 11;
        if rest == 10 {
            0
        } else {
            rest
        }
    };
    if digits[9] != check(9) || digits[10] != check(10) {
        return Err(InvalidCnpj);
    }
    Ok(Some(
        digits
            .iter()
            .map(|d| char::from_digit(*d, 10).unwrap_or('0'))
            .collect(),
    ))
}

pub const UFS: [&str; 27] = [
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA", "PB", "PR",
    "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

/// UF em maiúsculas se válida.
pub fn uf(value: &str) -> Option<String> {
    let v = value.trim().to_uppercase();
    UFS.contains(&v.as_str()).then_some(v)
}

/// Texto opcional: `None` se vazio, senão sem espaços nas pontas.
pub fn opt_text(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
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
    fn document_rules() {
        assert_eq!(
            document(Some("529.982.247-25")),
            Ok(Some("52998224725".into()))
        );
        assert!(document(Some("529.982.247-24")).is_err());
        assert!(document(Some("111.111.111-11")).is_err());
        assert_eq!(
            document(Some("11.222.333/0001-81")),
            Ok(Some("11222333000181".into()))
        );
        assert_eq!(document(Some("")), Ok(None));
        assert_eq!(uf(" sp ").as_deref(), Some("SP"));
        assert!(uf("XX").is_none());
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
