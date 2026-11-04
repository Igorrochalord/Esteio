# Contas de teste das integrações

Cada serviço on-line exige uma conta **sua** (CPF/CNPJ, e-mail, aceite de termos), então o
cadastro não pode ser automatizado. Este guia mostra onde criar cada conta de teste, o que
copiar para a tela **Integrações** e as limitações de cada ambiente.

> **Quer testar agora, sem conta nenhuma?** Use o **Simulador** (próxima seção).

## Simulador (sem conta, sem internet)

1. Entre como dono ou administrador → **Integrações** → **Simulador** → **Conectar**.
2. Ambiente **Teste** (é o único disponível) → **Salvar e testar**.
3. Opcional: "Segundos até o PIX ser pago" (padrão 20; `0` paga na hora).

O Simulador gera um PIX fictício (copia-e-cola no formato do BR Code, com chave
`simulador@nucleoerp.invalid`) e o marca como pago depois do tempo escolhido. Nada sai do
computador. Serve para testar o fluxo e para demonstrações.

## Resumo

| Serviço | Precisa de CNPJ? | Ambiente de teste | Custo |
|---|---|---|---|
| Mercado Pago (PIX) | não (CPF serve) | credenciais e usuários de teste | grátis |
| Asaas (PIX) | não no sandbox | sandbox separado | grátis |
| Efí (PIX) | sim (conta PJ Efí) | homologação, certificado próprio | grátis para testar |
| Mercado Livre | não | **sem sandbox**: usuários de teste em produção | grátis |
| Nuvemshop | não para parceiro | loja demo | grátis |
| Shopee | sim (aprovação de parceiro) | sandbox + loja de teste | grátis |
| Amazon SP-API | sim (Seller Central + perfil de desenvolvedor) | sandbox com respostas fixas | o plano de vendedor pode ser pago |
| NFS-e Nacional | **sim + certificado A1** | produção restrita | o A1 é pago (~1 ano) |

## PIX

### Mercado Pago

1. Crie (ou use) uma conta em [mercadopago.com.br](https://www.mercadopago.com.br).
2. Abra [Mercado Pago Developers → Suas integrações](https://www.mercadopago.com.br/developers/panel/app)
   e crie uma aplicação (pagamentos on-line).
3. Em **Credenciais de teste**, copie o **Access Token** de teste (começa com `APP_USR` ou `TEST`).
4. Para pagar como comprador, crie **usuários de teste** no mesmo painel (um vendedor e um comprador).
5. Documentação do teste de PIX:
   [testar a integração com Pix](https://www.mercadopago.com.br/developers/pt/docs/checkout-api-v2/integration-test/pix).

No Núcleo: Integrações → Mercado Pago → ambiente **Teste** → cole o Access Token.

### Asaas

1. Crie uma conta **no sandbox**, mesmo que já tenha conta real:
   [sandbox.asaas.com](https://sandbox.asaas.com). Cada ambiente tem a própria chave de API.
2. Em **Integrações → Chave de API**, gere a chave do sandbox.
3. **Cadastre uma chave Pix** na conta do sandbox (sem ela o QR Code não é registrado e o
   pagamento dá erro 404). Veja o [FAQ do sandbox](https://docs.asaas.com/docs/faq-sandbox) e
   [o que pode ser testado](https://docs.asaas.com/docs/o-que-pode-ser-testado).

Limitações: descontos/juros/multa em Pix e alguns layouts não são simulados.

### Efí (Efí Bank)

1. Abra uma conta Efí (PJ).
2. Na conta: **API → Aplicações → Nova aplicação**, ative a **API Pix** e os escopos de
   **Homologação**.
3. Em **API → Meus certificados**, escolha **Homologação** e gere o certificado `.p12`.
4. Copie **Client ID** e **Client Secret** de homologação e cadastre uma chave Pix.
5. Documentação: [credenciais, certificado e autorização](https://dev.efipay.com.br/docs/api-pix/credenciais/).
   Homologação: `pix-h.api.efipay.com.br`; produção: `pix.api.efipay.com.br`.

No Núcleo: envie o arquivo `.p12` no campo do certificado (ele fica cifrado neste computador).

## Marketplaces e loja virtual

Os marketplaces autorizam pelo navegador (OAuth). O endereço de retorno do aplicativo deve ser
a página do Núcleo que mostra o código para copiar:

```
https://igorrochalord.github.io/Esteio/oauth/
```

Para ela existir, ative o GitHub Pages do repositório: **Settings → Pages → Branch `main`,
pasta `/docs`**. A página (`docs/oauth/index.html`) é estática e não guarda nada.

### Mercado Livre

1. Crie a conta de desenvolvedor em
   [developers.mercadolivre.com.br](https://developers.mercadolivre.com.br) e crie um aplicativo
   (**Minhas aplicações**). Em *URI de redirect*, use o endereço acima.
2. Anote o **App ID** e a **Secret Key**.
3. **Não há sandbox**: os testes acontecem em produção com **usuários de teste**. Com um access
   token, crie até 10 usuários de teste por conta com `POST /users/test_user` (`{"site_id":"MLB"}`).
   Guarde login e senha na hora, porque não dá para recuperar depois. Usuários de teste só negociam
   com outros usuários de teste e somem após ~60 dias sem uso.
   Guia oficial: [realização de testes](https://developers.mercadolivre.com.br/pt_br/realizacao-de-testes).
4. Anuncie um produto com um usuário vendedor de teste e compre com o comprador de teste.

### Nuvemshop

1. Cadastre-se no [portal de parceiros](https://partners.nuvemshop.com.br) e crie um aplicativo
   (**Aplicativos → Criar aplicativo**, distribuição "para seus clientes"). O endereço de
   redirecionamento é o de cima.
2. Peça/crie uma **loja demo** (grátis, todas as funções, sem vendas reais).
3. Anote **App ID** (client_id) e **Client Secret**.
   Documentação: [aplicativos](https://dev.nuvemshop.com.br/docs/applications/overview) e
   [autenticação](https://dev.nuvemshop.com.br/docs/applications/authentication).

### Shopee

1. Cadastre-se no [Shopee Open Platform](https://open.shopee.com) como desenvolvedor/parceiro
   (passa por aprovação).
2. Crie um app; ele começa em **Developing** com **Partner ID** e **Partner Key de teste**,
   válidos só no sandbox (`partner.test-stable.shopeemobile.com`).
3. Em **API Test Tool**, crie/autorize uma **loja de teste**.
4. Cuidado: chave de teste com o endereço de produção (ou o contrário) dá `error_sign` /
   `invalid_partner_id`.

### Amazon (SP-API)

1. Tenha uma conta **Seller Central** (vendedor) no Brasil.
2. Registre-se como desenvolvedor no **Solution Provider Portal**
   ([visão geral](https://developer-docs.amazon.com/sp-api/docs/sp-api-registration-overview)).
   Pede dados da empresa.
3. O **aplicativo de sandbox** pode ser registrado enquanto o perfil está em análise
   ([passo 4](https://developer-docs.amazon.com/sp-api/docs/onboarding-step-4-register-your-first-sandbox-application)).
   O sandbox devolve respostas fixas.
4. O Brasil usa o endpoint **América do Norte** (`sellingpartnerapi-na.amazon.com`).

## NFS-e (Emissor Nacional)

1. **Certificado digital A1** (arquivo `.pfx`/`.p12`) do CNPJ que vai emitir, comprado de uma
   autoridade certificadora. É pago e vale cerca de 1 ano.
2. O **município** do prestador precisa ter aderido ao padrão nacional.
3. Os testes usam o ambiente **produção restrita** (homologação) do Sistema Nacional NFS-e, com o
   mesmo certificado.
4. Documentação: portal [gov.br/nfse](https://www.gov.br/nfse) (APIs e notas técnicas).

## Onde cada dado fica no Núcleo

- Texto comum (ex.: App ID) → configuração da integração.
- Tokens, senhas e certificados → **cifrados** neste computador, com a chave fora do banco e
  fora do backup ([backup](backup.md#integrações)). Nunca aparecem no histórico.
- Uma cópia restaurada em outro computador pede as credenciais de novo.
