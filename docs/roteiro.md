# Roteiro

O projeto avança em fases. Ao fim de cada fase, `make check` precisa estar verde, a documentação
é atualizada e o trabalho para até a aprovação para seguir.

| Fase | Escopo | Estado |
|---|---|---|
| 1 | Fundação SaaS (FastAPI + Postgres): substituída, preservada na tag `v0.1-saas` | substituída |
| 1b | Fundação local: núcleo Rust + SQLite, login, autocadastro, pessoas, empresa, histórico, backup, CI | em andamento |
| 2 | Comercial: produtos, categorias, estoque (movimentações imutáveis, mínimo), clientes, vendas com cancelamento, relatórios (vendas, ABC, estoque, mapa por UF, vendedores), grades tipo planilha, exportar/importar Excel | em validação |
| 3 | Design moderno: modo escuro, ícones, animações, Início como painel, comparação de períodos, imprimir/PDF | em validação |
| 4 | Financeiro: contas bancárias, a pagar/receber (fiado vira conta a receber), fluxo de caixa, DRE, extrato OFX | em validação |
| 5 | Ordens de serviço: quadro (arrastar entre etapas) e lista, serviços e peças, técnico e prazos, linha do tempo, faturar (venda + estoque + financeiro), imprimir OS | em validação |
| 6a | Base de integrações online (ADR 0003): fila tolerante a falta de internet, credenciais cifradas, tela Integrações, indicador de conexão | em validação |
| 6b | PIX na venda (Mercado Pago, Asaas, Efí): QR Code e baixa automática ao confirmar o pagamento | em validação |
| 6c | NFS-e pelo Emissor Nacional (gov.br; certificado A1) | — |
| 6d | Marketplaces e loja virtual: pedidos viram vendas e baixam o estoque | — |
| 7 | Planos e cobrança (limites já definidos em `plans.rs`; servidor de licença) | — |
| — | Ideias de mercado para avaliar depois: CRM/negociações, tarefas e aprovações, chamados, consolidação de planilhas, detecção de inconsistências | — |
| — | Sincronização entre filiais (serviço na nuvem + consumo do `change_log`) | — |
| — | Release: assinatura de código, atualização automática | — |

## Fase 1b: pendências

- [x] Compilar o app Tauri no WSL (`make check` verde, `make dev` abre).
- [ ] Roteiro manual na interface: onboarding → login → pedir cadastro → aprovar → redefinir senha →
      cores/logo → histórico → backup/restauração → fechar e reabrir.
- [ ] Primeiro instalador gerado pelo CI e testado numa máquina Windows limpa.

## Fase 6: integrações online

Decisão em [ADR 0003](adr/0003-integracoes-online.md): o sistema usa a internet, mas continua
abrindo e vendendo sem ela. Cada subfase termina com `make check` verde e validação na interface.

- [x] **6a, base**: fila `integration_jobs` (concluído / aguardando / repete com espera crescente /
      falha que pede ação), credenciais cifradas com a chave fora do banco e do backup, contrato
      `Provider`, tela **Integrações** (serviços, testar conexão, fila com tentar de novo/cancelar),
      aviso de conexão no menu e passo no tutorial.
- [x] **6b-0, infraestrutura comum**: tokens renovados gravados cifrados, trabalho recorrente com
      estado (`Wait` + payload), `run_job` (rodar agora), campos de arquivo + certificado `.p12` (mTLS),
      autorização pelo navegador (OAuth + `docs/oauth/`), **Simulador** e o guia
      [contas de teste](contas-de-teste.md).
- [x] **6b, PIX**: Mercado Pago (`/v1/payments`), Asaas (cobrança PIX + QR dinâmico) e Efí (API Pix
      do Banco Central com mTLS). Na venda em Pix, escolhe-se o serviço (ou "já recebi"); a venda vira
      **a receber** e a cobrança (`pix_charges`) entra na fila. O QR é desenhado no núcleo a partir do
      copia-e-cola; a tela confere a cada 4 s e a fila continua em segundo plano. Pago → baixa
      automática na conta padrão; vencido → cancela no serviço e permite gerar outro; venda
      cancelada → cancela a cobrança. Aguardando validação com as contas de teste.
- [ ] Depois da validação: escolher a conta que recebe cada serviço (hoje é a conta padrão) e
      devolução (estorno) do PIX pela tela.
- [ ] **6c, NFS-e**: Emissor Nacional (API, certificado A1 com mTLS, homologação), emissão a partir
      da OS/venda, cancelamento, limites do plano (`plans::ensure_within`).
- [ ] **6d, marketplaces**: Mercado Livre, Nuvemshop, Shopee e Amazon; pedidos por consulta viram
      vendas (estoque + a receber), correspondência por SKU. Vem antes da 6c (não depende de A1).
