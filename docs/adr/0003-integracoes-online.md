# ADR 0003: Integrações online opcionais

- Status: aceita
- Data: 2026-11-04
- Complementa: [ADR 0002](0002-executavel-local.md) (continua valendo: tudo local, uma empresa por instalação)

## Contexto

O ADR 0002 deixou o sistema 100% local, sem internet. Recursos que o mercado espera, como PIX
na venda, NFS-e e pedidos de marketplaces, dependem de APIs de terceiros. O dono do produto
decidiu que o sistema **usa a internet**, mas **continua funcionando sem ela**, só perdendo
essas funções enquanto estiver desconectado.

## Decisões

1. **Só chamadas de saída.** O núcleo (Rust) chama as APIs; não há servidor nosso, porta aberta
   nem webhook. Quando o provedor só avisa por webhook, a integração **consulta** de tempos em
   tempos (ex.: "o PIX já foi pago?"). A interface continua sem acesso à rede (CSP inalterada):
   o HTTP vive no núcleo, com `reqwest` + `rustls`, sem OpenSSL no instalador.

2. **Fila de trabalhos (`integration_jobs`).** Toda chamada que nasce de uma ação do usuário
   (ex.: gerar a cobrança de uma venda) entra na fila **na mesma transação** da ação, então a
   venda nunca fica esperando a internet. Um processador em segundo plano (a cada 15 s, mais o
   botão "Enviar agora") executa o que venceu:
   - `Done` grava o resultado e aplica o efeito local (`Provider::on_done`) numa transação;
   - `Wait` reagenda sem contar falha (ex.: PIX aguardando pagamento);
   - erro transitório (sem internet, 5xx, 429) repete com espera crescente (30 s → 1 h),
     até 8 vezes; erro definitivo (dados recusados, credencial inválida) falha na hora e
     aparece na tela para a pessoa tentar de novo ou cancelar;
   - trabalho que ficou "em execução" com o app fechado volta para a fila em 10 min.

3. **Provedores plugáveis.** Cada serviço implementa `Provider` (`info`, `check`, `run`,
   `on_done`) e é registrado no `Core`. A tela de Integrações lista os disponíveis e os
   planejados ("em breve").

4. **Credenciais cifradas, chave fora do banco.** Tokens e senhas de API vão para
   `integrations.secrets` cifrados com ChaCha20-Poly1305. A chave fica em
   `<pasta de dados>/integracoes.key` (0600 no Unix) e **não entra no backup**. Uma cópia
   restaurada em outro computador traz a configuração, mas pede as credenciais de novo. Segredos
   nunca vão para a auditoria nem voltam para a interface (só a lista de quais estão guardados).

5. **Permissão própria** (`Module::Integrations`): proprietário e administrador configuram;
   usuário não vê. O uso no dia a dia (ex.: gerar PIX numa venda) passa pela permissão do
   módulo de origem.

## Consequências

- O sistema passa a ter dependências externas por recurso, e cada provedor novo precisa de
  pesquisa (preço, homologação, limites, se funciona por consulta) antes da implementação.
- Sem webhook, a confirmação de pagamento tem atraso de alguns segundos (intervalo da consulta).
  Se um provedor essencial exigir webhook, será preciso um serviço na nuvem; isso pede um novo ADR.
- A fila gera linhas no `change_log` a cada mudança de situação. O sync futuro pode ignorar
  `integration_jobs`, que é estado local de cada instalação.

## Complemento (6b-0, 2026-11-04)

6. **Tokens que mudam.** O provedor pode renovar credenciais durante uma chamada
   (`Call::set_secrets`, ex.: refresh de OAuth). O serviço cifra e grava logo depois, sem
   auditar valores.
7. **OAuth sem servidor.** O endereço de retorno dos aplicativos é uma página estática
   (`docs/oauth/index.html`, servida pelo GitHub Pages) que só mostra o código para copiar.
   O núcleo abre o navegador (`tauri-plugin-opener`, chamado só pelo Rust, sem permissão na
   interface), a pessoa cola o código e o provedor troca por tokens.
   **Limitação aceita:** por enquanto o client_id/secret do aplicativo de cada marketplace é
   digitado na tela. Quando existir o servidor de licença (Fase 7), a troca de código passa por
   ele e o segredo do aplicativo sai das instalações.
8. **Certificados (mTLS).** Campos do tipo arquivo (`FieldKind::File`) guardam `.p12/.pfx` cifrados;
   `Http::with_pkcs12` converte para PEM (crate `p12-keystore`, Rust puro, aceita o formato
   legado 3DES dos A1) e monta um cliente rustls com identidade. Serve para a Efí e a NFS-e.
9. **Simulador.** Provedor embutido, só no ambiente de teste, que finge o PIX sem rede. Permite
   testar e demonstrar o fluxo sem conta nos serviços reais.
