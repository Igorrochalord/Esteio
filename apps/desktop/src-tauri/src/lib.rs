// O app desktop é só interface: não embute backend nem banco.
// Toda regra de negócio vive na API (URL configurável na tela de login).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Núcleo ERP");
}
