# Roteiro

O projeto avança em fases. Ao fim de cada fase, `make check` precisa estar verde, a documentação
é atualizada e o trabalho para até a aprovação para seguir.

| Fase | Escopo | Estado |
|---|---|---|
| 1 | Fundação SaaS (FastAPI + Postgres): substituída, preservada na tag `v0.1-saas` | substituída |
| 1b | Fundação local: núcleo Rust + SQLite, login, autocadastro, pessoas, empresa, histórico, backup, CI | em andamento |
| 2 | Comercial: produtos, categorias, estoque (movimentações imutáveis, mínimo), clientes, vendas com cancelamento, relatórios (vendas, ABC, estoque, mapa por UF, vendedores), grades tipo planilha, exportar/importar Excel | em validação |
| 3 | PDV de balcão (caixa: abertura/fechamento, troco), fornecedores e compras | — |
| 4 | Financeiro (contas a receber do fiado, a pagar, fluxo de caixa) | — |
| 5 | Relatórios adicionais e comparativos entre períodos | — |
| 6 | Sincronização entre filiais (serviço na nuvem + consumo do `change_log`) | — |
| 7 | Release: assinatura de código, atualização automática | — |
| — | Assinatura e licenciamento ([pesquisa](assinatura.md)): encaixar quando os planos forem decididos | proposta |

## Fase 1b: pendências

- [x] Compilar o app Tauri no WSL (`make check` verde, `make dev` abre).
- [ ] Roteiro manual na interface: onboarding → login → pedir cadastro → aprovar → redefinir senha →
      cores/logo → histórico → backup/restauração → fechar e reabrir.
- [ ] Primeiro instalador gerado pelo CI e testado numa máquina Windows limpa.
