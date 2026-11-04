//! Simulador: um provedor de mentira, só no ambiente de teste, para testar e demonstrar o fluxo
//! inteiro (PIX gerado → pago) sem conta em lugar nenhum e sem internet.
//!
//! Trabalhos que ele entende:
//! - `pix.create` → devolve um copia-e-cola fictício e o prazo;
//! - `pix.poll` → "pago" depois de `segundos_para_pagar` (padrão 20) contados de `created_at`
//!   do payload, ou na hora se o payload tiver `"simular_pago": true`.

use async_trait::async_trait;
use serde_json::json;

use crate::modules::integrations::catalog::{Field, Kind, ProviderInfo};
use crate::modules::integrations::models::Job;
use crate::modules::integrations::provider::{Call, Outcome, Provider, ProviderError};

pub const ID: &str = "simulador";
const DEFAULT_PAY_AFTER: i64 = 20;
/// validade do PIX simulado
const EXPIRES_MIN: i64 = 30;

pub struct Simulador;

fn pay_after(call: &Call<'_>) -> i64 {
    call.config_str("segundos_para_pagar")
        .and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|n| (0..=3600).contains(n))
        .unwrap_or(DEFAULT_PAY_AFTER)
}

fn iso(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

/// Copia-e-cola no formato do PIX (BR Code), com dados claramente fictícios.
fn fake_copy_paste(reference: &str, amount_cents: i64) -> String {
    let amount = format!("{}.{:02}", amount_cents / 100, amount_cents % 100);
    let field = |id: &str, v: &str| format!("{id}{:02}{v}", v.len());
    let account = format!(
        "{}{}",
        field("00", "br.gov.bcb.pix"),
        field("01", "simulador@nucleoerp.invalid")
    );
    let mut body = String::new();
    body += &field("00", "01");
    body += &field("26", &account);
    body += &field("52", "0000");
    body += &field("53", "986");
    body += &field("54", &amount);
    body += &field("58", "BR");
    body += &field("59", "SIMULADOR NUCLEO ERP");
    body += &field("60", "TESTE");
    let txid: String = reference
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(25)
        .collect();
    body += &field("62", &field("05", &txid));
    body += "6304";
    format!("{body}{:04X}", crc16_ccitt(body.as_bytes()))
}

/// CRC16-CCITT (0x1021, início 0xFFFF), o mesmo do BR Code.
fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[async_trait]
impl Provider for Simulador {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            fields: vec![Field::text("segundos_para_pagar", "Segundos até o PIX ser pago", false)
                .help("Padrão: 20. Use 0 para pagar na hora.")],
            capabilities: vec!["pix"],
            sandbox_only: true,
            ..ProviderInfo::new(
                ID,
                "Simulador",
                Kind::Other,
                "Teste o PIX na venda sem conta e sem internet. Nada é enviado para fora; o pagamento é fictício.",
            )
        }
    }

    async fn check(&self, call: &Call<'_>) -> Result<String, ProviderError> {
        if call.production {
            return Err(ProviderError::Rejected(
                "o Simulador só funciona no ambiente de teste".into(),
            ));
        }
        Ok(format!(
            "Simulador pronto: o PIX é \"pago\" {} segundos depois de gerado.",
            pay_after(call)
        ))
    }

    async fn run(&self, call: &Call<'_>, job: &Job) -> Outcome {
        if call.production {
            return Outcome::Error(ProviderError::Rejected(
                "o Simulador só funciona no ambiente de teste".into(),
            ));
        }
        let now = chrono::Utc::now();
        match job.kind.as_str() {
            "pix.create" => {
                let amount = job.payload["amount_cents"].as_i64().unwrap_or(0);
                if amount <= 0 {
                    return Outcome::Error(ProviderError::Rejected("valor inválido".into()));
                }
                let reference = format!("SIM{}", job.id.replace('-', ""));
                Outcome::Done(json!({
                    "provider_ref": reference,
                    "copy_paste": fake_copy_paste(&reference, amount),
                    "expires_at": iso(now + chrono::Duration::minutes(EXPIRES_MIN)),
                }))
            }
            "pix.poll" => {
                let forced = job.payload["simular_pago"].as_bool().unwrap_or(false);
                let created = job.payload["created_at"]
                    .as_str()
                    .and_then(parse)
                    .unwrap_or(now);
                let elapsed = (now - created).num_seconds();
                let wait = pay_after(call) - elapsed;
                if forced || wait <= 0 {
                    Outcome::Done(json!({ "status": "paid", "paid_at": iso(now) }))
                } else {
                    Outcome::Wait {
                        secs: wait.clamp(1, 5),
                        note: Some("Aguardando pagamento (simulado)".into()),
                        payload: None,
                    }
                }
            }
            "pix.cancel" => Outcome::Done(json!({ "status": "canceled" })),
            other => Outcome::Error(ProviderError::Rejected(format!(
                "o Simulador não sabe fazer \"{other}\""
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn br_code_has_valid_crc_and_amount() {
        let code = fake_copy_paste("SIM123", 1990);
        assert!(code.starts_with("000201"));
        assert!(code.contains("540519.90"));
        let (body, crc) = code.split_at(code.len() - 4);
        assert_eq!(crc, format!("{:04X}", crc16_ccitt(body.as_bytes())));
        // vetor conhecido do CRC16-CCITT-FALSE
        assert_eq!(crc16_ccitt(b"123456789"), 0x29B1);
    }
}
