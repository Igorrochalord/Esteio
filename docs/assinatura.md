# Plano de assinatura: pesquisa e recomendação

Status: **proposta** (nada implementado). Quando os planos e preços forem decididos, a decisão
vira um ADR e o licenciamento entra como fase própria no [roteiro](roteiro.md).

## O desafio específico do Núcleo

Num SaaS, o servidor simplesmente bloqueia quem não pagou. O Núcleo roda **no computador do
cliente, sem internet**, então a assinatura precisa de uma **licença que o programa consiga
conferir sozinho**, e de um jeito de renová-la quando houver conexão.

## Modelos de preço

| Modelo | Como funciona | Vantagens | Desvantagens |
|---|---|---|---|
| Preço único (flat) | um valor, tudo liberado | simples de vender e explicar; receita previsível | não cresce junto com o cliente; o pequeno acha caro e o grande paga pouco |
| Por usuário | valor × pessoas com acesso | acompanha o tamanho do cliente | incentiva compartilhar senha; o custo assusta conforme a equipe cresce |
| Por planos (tiers) | Essencial / Profissional / … com módulos e limites diferentes | atende vários portes; caminho natural de upgrade | mais trabalho para definir e manter; planos demais confundem |
| Por filial / CNPJ | valor por loja que usa o sistema | combina com o sync entre filiais (fase 6) | só faz sentido depois do sync |
| Híbrido | plano base + adicional por filial ou usuário extra | equilibra previsibilidade e crescimento | exige mais explicação na venda |

Os números públicos são de blogs de fornecedores e servem só como ordem de grandeza. Um deles
cita análise da Price Intelligently segundo a qual empresas com planos têm **ARPU ~44% maior**
que as de preço único ([Monetizely][monetizely-tier]). Muitas empresas acabam combinando
modelos ([Orb][orb]).

**Para o Núcleo:** planos por módulos (o ERP já é modular) com limite de usuários, e depois
adicional por filial quando a sincronização existir. Exemplo de desenho, sem preços:

| Plano | Para quem | Módulos | Limites |
|---|---|---|---|
| Essencial | MEI / loja pequena | cadastros, estoque, vendas | 3 usuários, 1 computador |
| Profissional | PME | + financeiro, relatórios | 10 usuários, 3 computadores |
| Rede | empresas com filiais | tudo + sincronização | por filial |

## Teste grátis × freemium

| | Teste grátis (trial) | Freemium |
|---|---|---|
| Conversão em pagante (faixa citada) | ~15–25% em testes bem desenhados | ~2–5% |
| Custo | só durante o teste | suporte contínuo para quem nunca paga |
| Combina com ERP? | sim: o cliente coloca os dados reais e sente o valor | pouco: ERP não é produto viral |

Faixas de [Monetizely][monetizely-free]; são números de blog, não estudo auditado.

**Recomendação:** teste de 14 a 30 dias **sem cartão**, com tudo liberado. Ao fim do teste ou
quando a assinatura vence, o sistema **não apaga nem sequestra dados**: entra em **modo somente
leitura**, com consulta, relatórios e backup sempre disponíveis. Isso evita atrito jurídico (os dados são do cliente,
LGPD) e preserva a confiança.

## Mensal × anual

- **Mensal:** entrada barata, mas maior cancelamento e mais cobranças falhando.
- **Anual com desconto** (prática comum: ~2 meses grátis): dinheiro antecipado, menos
  cancelamento e menos cobranças.
- Ofereça os dois; destaque o anual.

## Vantagens da assinatura (vs. vender a licença uma vez)

- **Receita recorrente e previsível**, o que facilita planejar e investir (e valoriza a empresa).
- **Entrada mais barata** para o cliente: pequenas empresas não pagam um valor alto de uma vez.
- **Atualizações contínuas** (NF-e, regras fiscais, melhorias) sem vender "versão nova".
- **Relação contínua:** suporte e upsell de módulos e filiais.

Custos e riscos: cancelamento (churn), inadimplência, necessidade de um **servidor de
licenças e cobrança** e alguns clientes que preferem pagar uma vez. Uma saída é oferecer
também uma licença perpétua com valor maior, mais um plano anual opcional de atualizações.

## Cobrança no Brasil

O cliente brasileiro paga muito por **Pix e boleto**, não só cartão. Exemplo pesquisado, o **Asaas**:

- Assinaturas com boleto, Pix ou cartão; o Asaas gera as cobranças recorrentes sozinho e avisa
  o seu sistema por **webhook** ([docs Asaas: assinaturas][asaas-assinaturas]).
- **Pix Automático**: o pagador autoriza uma vez e as cobranças seguintes são automáticas
  ([Pix Automático][asaas-pix-auto], [diferença para assinaturas][asaas-diff]). Exige conta PJ
  com CNPJ ativo há pelo menos 6 meses ([Pix Automático][asaas-pix-auto-docs]).
- Taxas: consultar a página oficial de preços na hora de decidir. Não achei tabela oficial
  atualizada na pesquisa.

Outras opções a comparar no mesmo critério (Pix, boleto, recorrência, webhook, taxa):
Mercado Pago, Pagar.me, Iugu e Stripe.

## Como a licença funcionaria no Núcleo

```
[gateway: Asaas]  --webhook: pagou-->  [servidor de licenças]  --licença assinada-->  [Núcleo no PC]
                                          (também será a base do sync da fase 6)
```

1. **Licença = arquivo assinado** (Ed25519) com: empresa/CNPJ, plano, módulos, limites
   (usuários, computadores) e **data de validade**. O programa traz a **chave pública** e
   confere a assinatura sem internet; ninguém consegue editar a licença sem invalidá-la.
2. **Renovação:** quando houver internet, o app consulta o servidor e baixa a licença renovada
   (ex.: validade = fim do período pago + carência).
3. **Carência offline** (ex.: 15 dias): um computador sem internet continua funcionando
   normalmente por um tempo.
4. **Vencida:** modo somente leitura, como na recomendação acima.
5. **Proteções simples:** amarrar a licença ao computador (fingerprint) e detectar relógio
   voltado para trás, comparando com a última data vista, gravada no banco.

**Fazer ou comprar o servidor de licenças?**

| Opção | Prós | Contras |
|---|---|---|
| **Keygen** (pronto, SaaS ou self-host) | licenças, máquinas, arquivos de licença offline; existe plugin para Tauri | mais um fornecedor e custo em dólar; ambientes totalmente offline pedem desenho extra ([Fitgap][fitgap-keygen], [plugin Tauri][tauri-keygen]) |
| **Serviço próprio pequeno** | controle total; reaproveitado no sync da fase 6 | precisa construir e operar (servidor, banco, segurança) |

**Recomendação:** começar com um serviço próprio mínimo (webhook do gateway → gera licença
assinada → endpoint de renovação), porque a fase 6 vai precisar de um servidor de qualquer forma.
No app, implementar um módulo `license` no núcleo que valida o arquivo e expõe o plano e os
limites para `require()`.

## Próximos passos (quando decidir)

1. Definir planos, limites e preços (mensal e anual) e a duração do teste.
2. Escolher o gateway (comparar Asaas, Mercado Pago, Pagar.me, Iugu e Stripe).
3. Registrar a decisão num ADR.
4. Implementar: módulo `license` no núcleo, tela "Assinatura", servidor de licenças e webhook.

## Fontes

- [Monetizely: tiered vs flat-rate][monetizely-tier]
- [Monetizely: freemium vs flat-rate][monetizely-free]
- [Orb: B2B SaaS pricing models][orb]
- [Asaas: assinaturas (recorrência)][asaas-assinaturas]
- [Asaas: Pix Automático][asaas-pix-auto] · [docs Pix Automático][asaas-pix-auto-docs] · [Pix Automático × assinaturas][asaas-diff]
- [Fitgap: Keygen][fitgap-keygen]
- [tauri-plugin-keygen (README)][tauri-keygen]

[monetizely-tier]: https://www.getmonetizely.com/articles/tiered-pricing-vs-flat-rate-pricing-choosing-the-right-strategy-for-saas-growth
[monetizely-free]: https://www.getmonetizely.com/articles/freemium-vs-flat-rate-pricing-which-model-drives-better-saas-growth
[orb]: https://withorb.com/blog/b2b-saas-pricing-models
[asaas-assinaturas]: https://docs.asaas.com/docs/assinaturas-recorrência
[asaas-pix-auto]: https://docs.asaas.com/reference/pix-automatico
[asaas-pix-auto-docs]: https://docs.asaas.com/docs/pix-automatico
[asaas-diff]: https://docs.asaas.com/docs/diferença-entre-pix-automático-e-assinaturas
[fitgap-keygen]: https://us.fitgap.com/products/042437/keygen
[tauri-keygen]: https://rust-digger.code-maven.com/src/tauri-plugin-keygen-0.1.0/README.md
