//! Seed de desenvolvimento: cria uma empresa fictícia com usuários e logo.
//! Passa pelos mesmos services do app (validação, auditoria, change_log). Idempotente.
//!
//! Uso: seed <pasta-de-dados> [padaria|techparts]

use nucleo_core::modules::company::models::{CompanyIn, OnboardingInput, OwnerIn};
use nucleo_core::modules::company::service as company;
use nucleo_core::modules::users::models::UserCreate;
use nucleo_core::modules::users::service as users;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, CoreError};

const PASSWORD: &str = "nucleo123";

struct Spec {
    name: &'static str,
    cnpj: Option<&'static str>,
    colors: (&'static str, &'static str),
    owner: (&'static str, &'static str),
    members: &'static [(&'static str, &'static str, &'static str)],
}

const PADARIA: Spec = Spec {
    name: "Padaria Pão Dourado",
    cnpj: Some("11.222.333/0001-81"),
    colors: ("#b45309", "#451a03"),
    owner: ("Joana Dourado", "dono@paodourado.com.br"),
    members: &[
        ("Carlos Admin", "admin@paodourado.com.br", "admin"),
        ("Bia Caixa", "caixa@paodourado.com.br", "user"),
    ],
};

const TECHPARTS: Spec = Spec {
    name: "TechParts Distribuidora",
    cnpj: None,
    colors: ("#0e7490", "#082f49"),
    owner: ("Rafael Tech", "dono@techparts.com.br"),
    members: &[("Lia Vendas", "vendas@techparts.com.br", "user")],
};

/// PNG de cor sólida sem dependências (zlib em blocos "stored", sem compressão).
fn solid_png(hex: &str, size: u32) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn chunk(out: &mut Vec<u8>, tag: &[u8], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = tag.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    }
    let rgb: Vec<u8> = (0..3)
        .map(|i| u8::from_str_radix(&hex[1 + 2 * i..3 + 2 * i], 16).unwrap_or(0))
        .collect();
    let mut raw = Vec::new();
    for _ in 0..size {
        raw.push(0); // filtro "none"
        raw.extend(rgb.repeat(size as usize));
    }
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65_535).enumerate() {
        let last = (i + 1) * 65_535 >= raw.len();
        z.push(u8::from(last));
        let len = block.len() as u16;
        z.extend(len.to_le_bytes());
        z.extend((!len).to_le_bytes());
        z.extend(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend(((b << 16) | a).to_be_bytes());

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend(size.to_be_bytes());
    ihdr.extend(size.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    png
}

async fn seed(core: &Core, spec: &Spec) -> Result<(), CoreError> {
    if company::status(core).await?.configured {
        println!("= já existe uma empresa nesta pasta; nada a fazer");
        return Ok(());
    }
    let input = OnboardingInput {
        company: CompanyIn {
            name: spec.name.into(),
            cnpj: spec.cnpj.map(Into::into),
            primary_color: spec.colors.0.into(),
            secondary_color: spec.colors.1.into(),
        },
        owner: OwnerIn {
            name: spec.owner.0.into(),
            email: spec.owner.1.into(),
            password: PASSWORD.into(),
        },
    };
    company::onboard(core, input).await?;
    println!("+ {}: criada (dono {})", spec.name, spec.owner.1);

    let ctx = core.require(Module::Settings, Action::Manage).await?;
    use base64::Engine;
    let logo = base64::engine::general_purpose::STANDARD.encode(solid_png(spec.colors.0, 64));
    company::upload_logo(&ctx, &logo).await?;

    for (name, email, role) in spec.members {
        let data = UserCreate {
            name: (*name).into(),
            email: (*email).into(),
            password: PASSWORD.into(),
            role: (*role).into(),
        };
        users::create(&ctx, data).await?;
        println!("  + usuário {email} ({role})");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next() else {
        eprintln!("uso: seed <pasta-de-dados> [padaria|techparts]");
        std::process::exit(2);
    };
    let spec = match args.next().as_deref() {
        None | Some("padaria") => &PADARIA,
        Some("techparts") => &TECHPARTS,
        Some(other) => {
            eprintln!("empresa desconhecida: {other} (use padaria ou techparts)");
            std::process::exit(2);
        }
    };
    let result = async {
        let core = Core::open(std::path::Path::new(&dir)).await?;
        seed(&core, spec).await
    }
    .await;
    match result {
        Ok(()) => println!("\nPasta: {dir}\nSenha de todos os usuários do seed: {PASSWORD}"),
        Err(e) => {
            eprintln!("falhou: {e} ({e:?})");
            std::process::exit(1);
        }
    }
}
