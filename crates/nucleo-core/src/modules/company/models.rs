use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::modules::double_option;

/// Empresa desta instalação (uma linha). O logo fica no próprio banco (backup = um arquivo).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Company {
    pub id: String,
    pub name: String,
    pub cnpj: Option<String>,
    pub primary_color: String,
    pub secondary_color: String,
    #[serde(skip_serializing)]
    pub logo: Option<Vec<u8>>,
    #[serde(skip_serializing)]
    pub logo_type: Option<String>,
    /// muda a cada envio: é o que a auditoria enxerga da troca de logo
    pub logo_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Company {
    /// Logo como data URL, pronto para `<img src>`.
    pub fn logo_url(&self) -> Option<String> {
        let (bytes, ty) = (self.logo.as_ref()?, self.logo_type.as_ref()?);
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        Some(format!("data:{ty};base64,{b64}"))
    }

    pub fn branding(&self) -> Branding {
        Branding {
            name: self.name.clone(),
            primary_color: self.primary_color.clone(),
            secondary_color: self.secondary_color.clone(),
            logo_url: self.logo_url(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Branding {
    pub name: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub logo_url: Option<String>,
}

/// Dados da empresa para a tela "Empresa".
#[derive(Debug, Clone, Serialize)]
pub struct CompanyOut {
    pub id: String,
    pub name: String,
    pub cnpj: Option<String>,
}

impl From<&Company> for CompanyOut {
    fn from(c: &Company) -> Self {
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            cnpj: c.cnpj.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SetupStatus {
    /// `false` na primeira abertura: o app mostra o onboarding
    pub configured: bool,
}

fn default_primary() -> String {
    "#2563eb".into()
}

fn default_secondary() -> String {
    "#0f172a".into()
}

#[derive(Debug, Deserialize)]
pub struct CompanyIn {
    pub name: String,
    pub cnpj: Option<String>,
    #[serde(default = "default_primary")]
    pub primary_color: String,
    #[serde(default = "default_secondary")]
    pub secondary_color: String,
}

#[derive(Debug, Deserialize)]
pub struct OwnerIn {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct OnboardingInput {
    pub company: CompanyIn,
    pub owner: OwnerIn,
}

#[derive(Debug, Default, Deserialize)]
pub struct CompanyUpdate {
    pub name: Option<String>,
    /// ausente = não mexe; `null` = remove
    #[serde(default, deserialize_with = "double_option")]
    pub cnpj: Option<Option<String>>,
}

#[derive(Debug, Default, Deserialize)]
pub struct BrandingUpdate {
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
}
