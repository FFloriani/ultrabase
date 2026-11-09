# Ultrabase — desenho da v0.1

Data: 09/11/2026. Aprovado pelo Floriani: "aprovo, qualquer coisa se eu não gostar a gente vai mexendo e editando".

## Objetivo

Um app de desktop (Windows) que organiza várias contas do Supabase num lugar só. Cada conta tem a própria janela de navegador, com login e cookies separados das outras, e o app mostra os projetos de cada conta, quantas vagas grátis sobram e quando cada projeto grátis vai pausar.

Feito para uso pessoal, no PC do Floriani; o código é público no GitHub (FFloriani/ultrabase) desde 09/11/2026.

## Fatos do Supabase que guiam o desenho

- São 2 projetos grátis por conta, contando todas as organizações onde a conta é dona ou admin. Organização extra na mesma conta não dá vaga nova. Fonte: https://supabase.com/docs/guides/platform/billing-on-supabase
- Projeto pausado não conta nas 2 vagas.
- Projeto grátis pausa depois de 1 semana sem uso. Fonte: https://supabase.com/pricing
- A API de gerenciamento (https://api.supabase.com/v1) aceita o token pessoal da conta (criado em https://supabase.com/dashboard/account/tokens). Endereços usados, conferidos na especificação oficial (https://api.supabase.com/api/v1-json) em 09/11/2026:
  - `GET /v1/profile` → e-mail e nome de usuário da conta
  - `GET /v1/organizations` e `GET /v1/organizations/{slug}` → organizações e plano (free, pro…)
  - `GET /v1/projects` → projetos, com `ref`, região, organização e status (`ACTIVE_HEALTHY`, `INACTIVE` = pausado, etc.)
  - `GET /v1/projects/{ref}/analytics/endpoints/usage.api-counts?interval=7day` → pedidos dos últimos 7 dias (base da estimativa de pausa)
  - `GET /v1/projects/{ref}/api-keys?reveal=true` → chaves do projeto
  - `POST /v1/projects/{ref}/pause` e `POST /v1/projects/{ref}/restore` → pausar e acordar

## O que entra na v0.1

Regra do dono (09/11/2026, depois do primeiro teste): a conta nasce do navegador. "o ultrabase tem que ter um sistema de browsers unicos pra cada 1 ter uma conta no supabase conectado".

1. **Nova conta = navegador novo:** um clique cria "Conta N" e abre na hora uma janela do navegador embutido do Windows (WebView2) com pasta de dados própria, já na página de login do Supabase. Nada é pedido antes.
2. **Login vigiado:** enquanto a janela está aberta, o app lê a cada 2,5 s o login que o painel guarda no navegador (`localStorage["supabase.dashboard.auth.token"]`, conferido no código aberto do painel em `packages/common/gotrue.ts`). Quando o login aparece, a conta vira "conectada" e o app lê tudo na hora.
3. **Sessão mantida:** o login fica na pasta da conta e continua depois de fechar o app. Para sincronizar com a janela fechada, o app abre a janela escondida, espera o painel renovar o login e fecha.
4. **Sincronizar:** com o login da conta (ou, de reserva, um token pessoal opcional), o app lê perfil, organizações, projetos e uso, e guarda uma cópia no banco local com a hora da leitura.
5. **Editar:** apelido, cor, e-mail, como entra, notas; senha e token ficam opcionais, no cofre do Windows (Gerenciador de Credenciais), nunca no banco nem em arquivo aberto.
6. **Painel:** cartão por conta com vagas usadas (x/2), status de cada projeto e dias estimados até a pausa. No topo, total de vagas livres e a conta sugerida para o próximo projeto.
7. **Ações por projeto:** acordar, pausar, abrir no painel do Supabase, copiar URL e chave pública (anon/publishable).

## Copiar para IA (pedido do dono em 09/11/2026)

"a ideia é facilmente eu aperta 1 botão e copiar tudo até as coisas + sigilosas para uma IDE com IA conseguir trabalhar".

- Cada projeto tem "Copiar para IA" e "Detalhes". O texto copiado tem: .env completo (URL, anon, publishable, service_role, secret, JWT secret, senha do banco, DATABASE_URL pelo pooler em modo transação, DIRECT_URL pelo pooler em modo sessão, conexão direta), a configuração do MCP do Supabase com o token, e avisos para a IA (o que é secreto, IPv6, Prisma, pausa do plano grátis).
- De onde vem cada coisa (rotas conferidas na captura do ReconLab e no código aberto do painel, `apps/studio/data`): chaves em `GET /v1/projects/{ref}/api-keys?reveal=true`; banco e JWT secret em `GET /platform/projects/{ref}/settings`; pooler em `GET /platform/projects/{ref}/config/supavisor` (com token pessoal, `GET /v1/projects/{ref}/config/database/pooler`).
- Senha do banco: o Supabase não guarda. O dono cola a dele (fica no cofre do Windows, nome `<conta>:db:<ref>`) ou pede uma nova: `PATCH /platform/projects/{ref}/db-password` com 32 letras e números aleatórios, guardada no cofre antes da troca.
- Token para o MCP: `POST /platform/profile/access-tokens` cria um token permanente "Ultrabase (<apelido>)", guardado no cofre; também serve de reserva para ler a conta.
- Na tela, segredos aparecem escondidos até "Mostrar"; os endereços do banco escondem a senha.

## Fora da v0.1

Proxy por conta, cadastro automático, backup exportável, ping automático contra a pausa, criar projeto pelo app.

## Peças

| Peça | O que faz | Depende de |
|---|---|---|
| `db` | Banco SQLite local: contas, organizações, projetos, uso, hora da última leitura | rusqlite |
| `vault` | Lê e grava segredos no Gerenciador de Credenciais do Windows | keyring |
| `supabase` | Cliente da API de gerenciamento, só chamadas e tipos | reqwest |
| `slots` | Regras puras: vagas usadas, dias até a pausa, conta sugerida | nada (testável sozinho) |
| `browser` | Abre/foca a janela isolada de cada conta | Tauri |
| `commands` | Liga tudo à interface (comandos Tauri) | todas acima |
| Interface | Svelte 5 + TypeScript: lista de contas, detalhe, formulários | comandos |

As janelas das contas carregam sites de fora e **não** têm acesso aos comandos do app: a permissão do app vale só para a janela principal.

## Como os dados andam

1. "Nova conta" → conta no banco (estado "esperando você entrar") e navegador aberto no login do Supabase.
2. O dono entra no Supabase nessa janela → o vigia vê o login, marca "conectada" e sincroniza.
3. "Sincronizar" → o Rust pega o login do navegador da conta (abrindo a janela escondida se precisar; o token do cofre é só reserva), chama a API, grava a cópia no banco e devolve o resumo para a tela.
4. A tela sempre lê do banco. Sem internet, mostra a última cópia com a hora.

## Estimativa de pausa

Dias até pausar = 7 − dias desde o último pedido que a API contou nos últimos 7 dias. Sem pedido nenhum em 7 dias → "pausa a qualquer momento". É estimativa: a API só conta pedidos pela API (REST, login, storage, realtime), não consulta direta no banco.

## Falhas

- Login recusado (401) → a conta fica "desconectada", com o botão para abrir o navegador e entrar de novo. O vigia não repete a mesma leitura recusada; só tenta de novo quando o login muda.
- Token pessoal recusado (401) → o token fica marcado "recusado".
- Limite de pedidos (429) → espera e mostra aviso; não repete sem parar.
- Sem internet → mantém a cópia antiga e mostra a hora dela.
- Cofre indisponível → erro claro; nunca grava o segredo em arquivo como plano B.

## Testes

- Automáticos (Rust): regras de vagas e de pausa, leitura das respostas da API a partir de exemplos gravados, banco em memória.
- Manual: abrir duas contas, logar numa e confirmar que a outra continua deslogada; sincronizar uma conta com token real.

## Conferido na construção (09/11/2026)

- A pasta de dados separada por janela (opção `data_directory` do Tauri 2) funciona: com duas contas abertas rodam 3 navegadores WebView2 separados (o do app e um por conta), cada um com a própria pasta e o próprio arquivo de cookies.
- Comandos que criam ou fecham janela precisam ser `async`: síncronos rodam na linha principal e travam o app no Windows.
- Ao apagar uma conta, o WebView2 ainda segura a pasta dela por alguns segundos; a limpeza ao abrir o app apaga o que sobrou.
- Token falso enviado à API real volta recusado (401) e a conta fica marcada "token recusado".
