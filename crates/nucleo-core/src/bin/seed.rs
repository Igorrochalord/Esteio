//! Seed de desenvolvimento: cria uma empresa fictícia com usuários e logo.
//! Passa pelos mesmos services do app (validação, auditoria, change_log). Idempotente.
//!
//! Uso: seed <pasta-de-dados> [padaria|techparts]

use nucleo_core::modules::auth::service as auth;
use nucleo_core::modules::company::models::{CompanyIn, OnboardingInput, OwnerIn};
use nucleo_core::modules::company::service as company;
use nucleo_core::modules::customers::models::CustomerInput;
use nucleo_core::modules::customers::service as customers;
use nucleo_core::modules::finance::models::{
    AccountInput, EntryInput, EntryQuery, PayInput, TransferInput,
};
use nucleo_core::modules::finance::service as fin;
use nucleo_core::modules::products::models::ProductInput;
use nucleo_core::modules::products::service as products;
use nucleo_core::modules::sales::models::{SaleInput, SaleItemInput};
use nucleo_core::modules::sales::service as sales;
use nucleo_core::modules::users::models::UserCreate;
use nucleo_core::modules::users::service as users;
use nucleo_core::modules::users::service::sellers;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, CoreError};
use rust_decimal::Decimal;

const PASSWORD: &str = "nucleo123";

/// (nome, categoria, unidade, preço, custo, controla estoque, mínimo, peso nas vendas)
type Item = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    bool,
    u32,
    u32,
);
/// (nome, cidade, UF)
type Client = (&'static str, &'static str, &'static str);

struct Spec {
    catalog: &'static [Item],
    clients: &'static [Client],
    /// vendas por dia (base)
    daily: u32,
    name: &'static str,
    cnpj: Option<&'static str>,
    colors: (&'static str, &'static str),
    owner: (&'static str, &'static str),
    members: &'static [(&'static str, &'static str, &'static str)],
}

const PADARIA: Spec = Spec {
    catalog: &[
        ("Pão francês", "Pães", "kg", "16.90", "6.20", true, 5, 30),
        (
            "Pão de forma integral",
            "Pães",
            "un",
            "12.50",
            "5.10",
            true,
            8,
            6,
        ),
        ("Pão de queijo", "Pães", "kg", "54.90", "21.00", true, 3, 10),
        ("Baguete", "Pães", "un", "9.90", "3.40", true, 10, 6),
        ("Croissant", "Pães", "un", "8.50", "3.10", true, 12, 8),
        (
            "Bolo de chocolate",
            "Doces",
            "un",
            "38.00",
            "15.50",
            true,
            3,
            4,
        ),
        (
            "Bolo de cenoura",
            "Doces",
            "un",
            "34.00",
            "13.20",
            true,
            3,
            3,
        ),
        ("Sonho", "Doces", "un", "6.50", "2.30", true, 15, 9),
        ("Brigadeiro", "Doces", "un", "3.50", "1.10", true, 40, 7),
        (
            "Torta de limão (fatia)",
            "Doces",
            "un",
            "12.00",
            "4.60",
            true,
            8,
            4,
        ),
        ("Coxinha", "Salgados", "un", "7.50", "2.40", true, 20, 12),
        (
            "Esfiha de carne",
            "Salgados",
            "un",
            "6.90",
            "2.20",
            true,
            20,
            8,
        ),
        (
            "Empada de frango",
            "Salgados",
            "un",
            "8.00",
            "2.90",
            true,
            15,
            6,
        ),
        (
            "Pão na chapa",
            "Salgados",
            "un",
            "6.00",
            "1.60",
            false,
            0,
            9,
        ),
        (
            "Misto quente",
            "Salgados",
            "un",
            "11.00",
            "4.10",
            false,
            0,
            7,
        ),
        (
            "Café expresso",
            "Bebidas",
            "un",
            "6.00",
            "1.20",
            false,
            0,
            14,
        ),
        ("Cappuccino", "Bebidas", "un", "9.50", "2.60", false, 0, 7),
        (
            "Suco de laranja 300ml",
            "Bebidas",
            "un",
            "9.00",
            "3.20",
            false,
            0,
            6,
        ),
        (
            "Refrigerante lata",
            "Bebidas",
            "un",
            "6.50",
            "3.40",
            true,
            24,
            8,
        ),
        (
            "Água mineral 500ml",
            "Bebidas",
            "un",
            "4.00",
            "1.30",
            true,
            24,
            6,
        ),
        (
            "Queijo mussarela",
            "Frios",
            "kg",
            "59.90",
            "36.00",
            true,
            2,
            5,
        ),
        ("Presunto", "Frios", "kg", "44.90", "27.50", true, 2, 4),
        ("Requeijão 200g", "Frios", "un", "11.90", "6.80", true, 6, 3),
        ("Manteiga 200g", "Frios", "un", "14.90", "8.90", true, 6, 3),
        (
            "Leite integral 1L",
            "Mercearia",
            "un",
            "6.29",
            "4.40",
            true,
            24,
            8,
        ),
        (
            "Café 500g",
            "Mercearia",
            "un",
            "28.90",
            "19.80",
            true,
            10,
            3,
        ),
        (
            "Achocolatado 400g",
            "Mercearia",
            "un",
            "11.50",
            "7.10",
            true,
            8,
            2,
        ),
        (
            "Ovos (dúzia)",
            "Mercearia",
            "un",
            "14.00",
            "9.50",
            true,
            10,
            3,
        ),
    ],
    clients: &[
        ("Maria Aparecida Silva", "Campinas", "SP"),
        ("João Batista Souza", "Campinas", "SP"),
        ("Ana Paula Ribeiro", "Valinhos", "SP"),
        ("Carlos Eduardo Lima", "São Paulo", "SP"),
        ("Fernanda Costa", "São Paulo", "SP"),
        ("Rafael Oliveira", "Jundiaí", "SP"),
        ("Juliana Martins", "Sorocaba", "SP"),
        ("Pedro Henrique Alves", "Belo Horizonte", "MG"),
        ("Luciana Ferreira", "Uberlândia", "MG"),
        ("Marcos Pereira", "Poços de Caldas", "MG"),
        ("Patrícia Gomes", "Rio de Janeiro", "RJ"),
        ("Roberto Carvalho", "Niterói", "RJ"),
        ("Camila Rocha", "Curitiba", "PR"),
        ("Bruno Teixeira", "Londrina", "PR"),
        ("Aline Barbosa", "Florianópolis", "SC"),
        ("Diego Araújo", "Porto Alegre", "RS"),
        ("Vanessa Cardoso", "Salvador", "BA"),
        ("Thiago Nascimento", "Recife", "PE"),
        ("Larissa Moreira", "Fortaleza", "CE"),
        ("Gustavo Mendes", "Goiânia", "GO"),
        ("Beatriz Castro", "Brasília", "DF"),
        ("Felipe Monteiro", "Vitória", "ES"),
        ("Condomínio Jardim das Flores", "Campinas", "SP"),
        ("Escola Pequeno Príncipe", "Campinas", "SP"),
    ],
    daily: 22,
    name: "Padaria Pão Dourado",
    cnpj: Some("11.222.333/0001-81"),
    colors: ("#b45309", "#451a03"),
    owner: ("Joana Dourado", "dono@paodourado.com.br"),
    members: &[
        ("Carlos Admin", "admin@paodourado.com.br", "admin"),
        ("Bia Caixa", "caixa@paodourado.com.br", "user"),
        ("Diego Balcão", "balcao@paodourado.com.br", "user"),
    ],
};

const TECHPARTS: Spec = Spec {
    catalog: &[
        (
            "Cabo HDMI 2m",
            "Cabos",
            "un",
            "29.90",
            "11.00",
            true,
            20,
            10,
        ),
        (
            "Cabo USB-C 1m",
            "Cabos",
            "un",
            "24.90",
            "8.50",
            true,
            30,
            12,
        ),
        (
            "Carregador 20W",
            "Energia",
            "un",
            "79.90",
            "38.00",
            true,
            10,
            8,
        ),
        (
            "Power bank 10000mAh",
            "Energia",
            "un",
            "129.90",
            "71.00",
            true,
            5,
            5,
        ),
        (
            "Mouse sem fio",
            "Periféricos",
            "un",
            "59.90",
            "27.00",
            true,
            10,
            8,
        ),
        (
            "Teclado USB",
            "Periféricos",
            "un",
            "89.90",
            "42.00",
            true,
            8,
            5,
        ),
        (
            "Headset",
            "Periféricos",
            "un",
            "149.90",
            "78.00",
            true,
            5,
            4,
        ),
        (
            "SSD 480GB",
            "Armazenamento",
            "un",
            "229.90",
            "158.00",
            true,
            5,
            4,
        ),
        (
            "Pendrive 64GB",
            "Armazenamento",
            "un",
            "39.90",
            "19.00",
            true,
            15,
            7,
        ),
        (
            "Instalação e configuração",
            "Serviços",
            "un",
            "120.00",
            "0",
            false,
            0,
            2,
        ),
    ],
    clients: &[
        ("Escritório Contábil Andrade", "São Paulo", "SP"),
        ("Clínica Bem Viver", "Santos", "SP"),
        ("Colégio Horizonte", "Belo Horizonte", "MG"),
        ("Loja Conecta", "Rio de Janeiro", "RJ"),
        ("Hotel Serra Azul", "Gramado", "RS"),
        ("Studio Pixel", "Curitiba", "PR"),
        ("Mercado Bom Preço", "Salvador", "BA"),
    ],
    daily: 9,
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
        println!("= empresa já existe nesta pasta");
    } else {
        seed_company(core, spec).await?;
    }
    auth::login(core, spec.owner.1, PASSWORD.into()).await?;
    seed_commerce(core, spec).await
}

async fn seed_company(core: &Core, spec: &Spec) -> Result<(), CoreError> {
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

/// Gerador pseudoaleatório simples e determinístico (mesmo seed → mesmos dados).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    /// índice sorteado por peso
    fn weighted(&mut self, weights: &[u32]) -> usize {
        let total: u64 = weights.iter().map(|&w| u64::from(w)).sum();
        let mut x = self.below(total);
        for (i, &w) in weights.iter().enumerate() {
            if x < u64::from(w) {
                return i;
            }
            x -= u64::from(w);
        }
        weights.len() - 1
    }
}

fn dec(s: &str) -> Decimal {
    s.parse().unwrap_or_default()
}

/// Catálogo, clientes e 90 dias de vendas com padrão realista (picos de manhã e fim da
/// tarde, fim de semana mais forte, mix de pagamentos, alguns cancelamentos).
async fn seed_commerce(core: &Core, spec: &Spec) -> Result<(), CoreError> {
    let ctx = core.require(Module::Products, Action::Manage).await?;
    if !products::list(&ctx).await?.is_empty() {
        println!("= produtos já existem; vendas de demonstração não foram geradas");
        return Ok(());
    }
    let mut catalog = Vec::new();
    for (name, category, unit, price, cost, track, min, weight) in spec.catalog {
        let cat = products::create_category(&ctx, category).await?;
        let p = products::create(
            &ctx,
            ProductInput {
                code: String::new(),
                barcode: None,
                name: (*name).into(),
                category_id: Some(cat.id),
                unit: Some((*unit).into()),
                price: dec(price),
                cost: Some(dec(cost)),
                track_stock: Some(*track),
                min_stock: Some(Decimal::from(*min)),
                // estoque para os 90 dias; alguns terminam abaixo do mínimo de propósito
                initial_stock: track.then(|| Decimal::from(*weight * spec.daily * 3 / 2)),
            },
        )
        .await?;
        catalog.push((p, *weight));
    }
    println!("  + {} produtos", catalog.len());

    let mut client_ids = Vec::new();
    for (i, (name, city, uf)) in spec.clients.iter().enumerate() {
        let c = customers::create(
            &ctx,
            CustomerInput {
                name: (*name).into(),
                document: None,
                phone: Some(format!("(19) 9{:04}-{:04}", 8000 + i, 1000 + i * 37)),
                email: None,
                city: Some((*city).into()),
                state: Some((*uf).into()),
                notes: None,
            },
        )
        .await?;
        client_ids.push(c.id);
    }
    println!("  + {} clientes", client_ids.len());

    // contas: o caixa recebe as vendas à vista
    let account = |name: &str, kind: &str, initial: i64, default: bool| AccountInput {
        name: name.into(),
        kind: kind.into(),
        bank_name: (kind != "caixa").then(|| name.split(' ').next().unwrap_or(name).to_owned()),
        agency: (kind == "corrente").then(|| "0341".into()),
        account_number: (kind == "corrente").then(|| "12345-6".into()),
        initial_balance: Some(Decimal::from(initial)),
        opening_date: Some(
            (chrono::Utc::now() - chrono::Duration::days(95))
                .format("%Y-%m-%d")
                .to_string(),
        ),
        is_default: Some(default),
    };
    let caixa = fin::create_account(&ctx, account("Caixa da loja", "caixa", 500, true)).await?;
    let banco = fin::create_account(&ctx, account("Itaú PJ", "corrente", 8000, false)).await?;
    fin::create_account(&ctx, account("Nubank PJ", "carteira", 1200, false)).await?;

    let seller_list = sellers(&ctx).await?;
    let seller_weights: Vec<u32> = (0..seller_list.len())
        .map(|i| [2, 3, 5, 4][i % 4])
        .collect();
    let weights: Vec<u32> = catalog.iter().map(|(_, w)| *w).collect();
    // clientes "fiéis" compram mais (peso decrescente)
    let client_weights: Vec<u32> = (0..client_ids.len())
        .map(|i| 30 / (1 + i as u32 / 3) + 1)
        .collect();
    let payments = ["pix", "debito", "credito", "dinheiro", "fiado"];
    let pay_weights = [40, 25, 15, 15, 5];
    let hour_weights = [
        0, 0, 0, 0, 0, 0, 3, 9, 10, 6, 4, 4, 5, 3, 2, 3, 5, 8, 9, 6, 3, 1, 0, 0,
    ];

    let mut rng = Rng(20_261_104);
    let today = (chrono::Utc::now() - chrono::Duration::hours(3)).date_naive();
    let mut count = 0;
    for days_ago in (0..90).rev() {
        let day = today - chrono::Duration::days(days_ago);
        use chrono::Datelike;
        let weekend = matches!(day.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun);
        // tendência leve de crescimento + fim de semana mais forte
        let base = spec.daily as u64 * (90 + (90 - days_ago as u64) / 3) / 100;
        let n = base * if weekend { 13 } else { 10 } / 10 + rng.below(6);
        for _ in 0..n {
            let hour = rng.weighted(&hour_weights) as u32;
            let local = day.and_hms_opt(hour, rng.below(60) as u32, rng.below(60) as u32);
            let Some(local) = local else { continue };
            if days_ago == 0
                && local > (chrono::Utc::now() - chrono::Duration::hours(3)).naive_utc()
            {
                continue; // não cria venda no futuro
            }
            let sold_at = (local + chrono::Duration::hours(3))
                .format("%Y-%m-%dT%H:%M:%S.000Z")
                .to_string();
            let mut payment = payments[rng.weighted(&pay_weights)];
            let customer = if payment == "fiado" || rng.below(100) < 45 {
                Some(client_ids[rng.weighted(&client_weights)].clone())
            } else {
                None
            };
            if customer.is_none() && payment == "fiado" {
                payment = "pix";
            }
            let lines = 1 + rng.weighted(&[50, 30, 15, 5]);
            let mut items: Vec<SaleItemInput> = Vec::new();
            for _ in 0..lines {
                let (p, _) = &catalog[rng.weighted(&weights)];
                if items.iter().any(|i| i.product_id == p.id) {
                    continue;
                }
                let qty = if p.unit == "kg" {
                    Decimal::new(150 + rng.below(700) as i64, 3) // 0,150 a 0,850 kg
                } else {
                    Decimal::from(1 + rng.weighted(&[60, 25, 10, 5]) as i64)
                };
                items.push(SaleItemInput {
                    product_id: p.id.clone(),
                    qty,
                    unit_price: None,
                    discount: None,
                });
            }
            let seller = &seller_list[rng.weighted(&seller_weights)];
            let input = SaleInput {
                customer_id: customer,
                seller_id: Some(seller.id.clone()),
                payment_method: payment.into(),
                discount: None,
                notes: None,
                sold_at: Some(sold_at),
                tz_offset_minutes: Some(-180),
                items,
            };
            match sales::create(&ctx, input).await {
                Ok(sale) => {
                    count += 1;
                    if rng.below(100) < 2 {
                        sales::cancel(&ctx, &sale.id, "Cancelada pelo cliente (demonstração)")
                            .await?;
                    }
                }
                // estoque acabou para algum item: pula essa venda
                Err(CoreError::BusinessRule(_)) => continue,
                Err(e) => return Err(e),
            }
        }
    }
    println!("  + {count} vendas em 90 dias");
    seed_finance(&ctx, &mut rng, &caixa.id, &banco.id, today).await?;
    Ok(())
}

/// Contas fixas, compras, depósitos do caixa no banco e recebimento da maior parte do fiado.
async fn seed_finance(
    ctx: &nucleo_core::Ctx,
    rng: &mut Rng,
    caixa: &str,
    banco: &str,
    today: chrono::NaiveDate,
) -> Result<(), CoreError> {
    use chrono::Datelike;
    let cats = fin::categories(ctx).await?;
    let cat = |name: &str| {
        cats.iter()
            .find(|c| c.name == name)
            .map(|c| c.id.clone())
            .unwrap_or_default()
    };
    let fmt = |d: chrono::NaiveDate| d.format("%Y-%m-%d").to_string();
    let first = today.with_day(1).unwrap_or(today);
    let mut entries = 0;
    // 3 meses passados + o atual + 2 futuros
    for back in -3i32..=2 {
        let month = if back < 0 {
            first - chrono::Months::new(back.unsigned_abs())
        } else {
            first + chrono::Months::new(back as u32)
        };
        let fixed = [
            (
                "Aluguel da loja",
                "Aluguel",
                2800,
                5,
                Some("Imobiliária Central"),
            ),
            ("Folha de pagamento", "Salários e encargos", 4800, 5, None),
            (
                "Energia elétrica",
                "Energia, água e internet",
                420 + rng.below(220) as i64,
                15,
                Some("CPFL"),
            ),
            (
                "Internet",
                "Energia, água e internet",
                149,
                10,
                Some("Vivo Empresas"),
            ),
            (
                "Simples Nacional (DAS)",
                "Impostos e taxas",
                1100 + rng.below(400) as i64,
                20,
                Some("Receita Federal"),
            ),
            (
                "Farinha e insumos",
                "Compras de mercadoria",
                1800 + rng.below(900) as i64,
                12,
                Some("Moinho Bom Grão"),
            ),
            (
                "Laticínios",
                "Compras de mercadoria",
                900 + rng.below(500) as i64,
                22,
                Some("Laticínios Serra"),
            ),
        ];
        for (desc, category, value, day, who) in fixed {
            let due = month.with_day(day).unwrap_or(month);
            let created = fin::create_entry(
                ctx,
                EntryInput {
                    kind: "pagar".into(),
                    description: desc.into(),
                    category_id: cat(category),
                    customer_id: None,
                    counterparty: who.map(Into::into),
                    amount: Decimal::from(value),
                    issue_date: Some(fmt(due)),
                    due_date: fmt(due),
                    installments: None,
                    repeat_months: None,
                    document: None,
                    notes: None,
                },
            )
            .await?;
            entries += 1;
            // vencidos são pagos, exceto dois deixados em atraso de propósito
            let leave_open =
                back == 0 && (desc == "Laticínios" || desc == "Energia elétrica") && due < today;
            if due < today && !leave_open {
                fin::pay_entry(
                    ctx,
                    &created[0].id,
                    PayInput {
                        date: fmt(due),
                        amount: None,
                        bank_account_id: Some(banco.into()),
                        payment_method: Some("pix".into()),
                    },
                )
                .await?;
            }
        }
    }
    // depósito semanal do caixa no banco (parte do dinheiro e cartões do caixa)
    let mut day = today - chrono::Duration::days(84);
    while day < today {
        // deposita o equivalente a uma semana de vendas
        let amount = Decimal::from(3800 + rng.below(900) as i64);
        fin::transfer(
            ctx,
            TransferInput {
                from_account_id: caixa.into(),
                to_account_id: banco.into(),
                date: fmt(day),
                amount,
                description: Some("Depósito do caixa".into()),
            },
        )
        .await?;
        day += chrono::Duration::days(7);
    }
    // fiado: ~80% dos vencidos já recebidos
    let fiado = fin::entries(
        ctx,
        EntryQuery {
            kind: Some("receber".into()),
            status: Some("aberto".into()),
            ..Default::default()
        },
    )
    .await?;
    let mut received = 0;
    for e in fiado {
        let due = chrono::NaiveDate::parse_from_str(&e.due_date, "%Y-%m-%d").unwrap_or(today);
        if due < today && rng.below(100) < 80 {
            let paid = due - chrono::Duration::days(rng.below(10) as i64);
            fin::pay_entry(
                ctx,
                &e.id,
                PayInput {
                    date: fmt(paid),
                    amount: None,
                    bank_account_id: Some(banco.into()),
                    payment_method: Some("pix".into()),
                },
            )
            .await?;
            received += 1;
        }
    }
    println!("  + financeiro: 3 contas, {entries} contas a pagar, depósitos semanais, {received} fiados recebidos");
    Ok(())
}
