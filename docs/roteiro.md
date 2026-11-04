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
| 6 | NFS-e pelo Emissor Nacional (gov.br; certificado A1; única parte online) | próxima |
| 7 | Planos e cobrança (limites já definidos em `plans.rs`; servidor de licença) | — |
| — | Ideias de mercado para avaliar depois: CRM/negociações, tarefas e aprovações, chamados, consolidação de planilhas, detecção de inconsistências | — |
| — | Sincronização entre filiais (serviço na nuvem + consumo do `change_log`) | — |
| — | Release: assinatura de código, atualização automática | — |

## Fase 1b: pendências

- [x] Compilar o app Tauri no WSL (`make check` verde, `make dev` abre).
- [ ] Roteiro manual na interface: onboarding → login → pedir cadastro → aprovar → redefinir senha →
      cores/logo → histórico → backup/restauração → fechar e reabrir.
- [ ] Primeiro instalador gerado pelo CI e testado numa máquina Windows limpa.
