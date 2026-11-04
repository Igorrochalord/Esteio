# Roteiro

O projeto avança em fases. Ao fim de cada fase, `make check` precisa estar verde, a documentação
é atualizada e o trabalho para até a aprovação para seguir.

| Fase | Escopo | Estado |
|---|---|---|
| 1 | Fundação SaaS (FastAPI + Postgres): substituída, preservada na tag `v0.1-saas` | substituída |
| 1b | Fundação local: núcleo Rust + SQLite, login, pessoas, empresa, histórico, backup, CI | em andamento |
| 2 | Estoque: categorias, fornecedores, produtos, movimentações imutáveis, alerta de mínimo | próxima |
| 3 | Vendas / PDV | — |
| 4 | Financeiro | — |
| 5 | Relatórios | — |
| 6 | Sincronização entre filiais (serviço na nuvem + consumo do `change_log`) | — |
| 7 | Release: assinatura de código, atualização automática | — |

## Fase 1b: pendências

- [x] Compilar o app Tauri no WSL (`make check` verde, `make dev` abre).
- [ ] Roteiro manual na interface: onboarding → login → pessoas → cores/logo → histórico →
      backup/restauração → fechar e reabrir.
- [ ] Primeiro instalador gerado pelo CI e testado numa máquina Windows limpa.
