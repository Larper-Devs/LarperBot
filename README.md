# LarperBot

Bot modular para Discord, escrito em TypeScript com `discord.js`, MongoDB/Mongoose e `@napi-rs/canvas`.

O projeto prioriza slash commands. Comandos por prefixo existem apenas para compatibilidade e são opcionais.

## Estado atual

- Migração para `discord.js` concluída.
- Carregamento automático de comandos e eventos.
- Slash commands registrados por servidor ou globalmente.
- Help por categoria usando Components V2.
- Select menu do Help dentro de um `Container` roxo.
- Apenas o autor do Help pode usar o menu de categorias.
- Comandos administrativos protegidos por `Administrator`.
- Moderação manual e automod.
- Economia própria com carteira, recompensas, loja, inventário e extrato.
- Níveis total, semanal e mensal.
- Perfil e ranking em canvas com identidade roxa e branca.
- Boas-vindas e painel de escolha de cores.
- Blackjack interativo, coinflip e enquete.
- Calculadora DevEx com autocomplete de moedas e cotação diária.

## Requisitos

- Node.js 20 ou superior.
- Corepack/pnpm.
- Aplicação e bot criados no [Discord Developer Portal](https://discord.com/developers/applications).
- MongoDB Atlas apenas para recursos persistentes.

## Instalação

```bash
corepack pnpm install
Copy-Item .env.example .env
```

Preencha pelo menos `DISCORD_TOKEN` e `CLIENT_ID` no `.env`.

### Desenvolvimento

```bash
corepack pnpm dev
```

### Build e produção

```bash
corepack pnpm build
corepack pnpm start
```

### Registro dos slash commands

O comando correto é:

```bash
corepack pnpm deploy:commands
```

Não use `pnpm deploy` sem alvo; esse é um comando nativo do pnpm e exige um parâmetro.

Com `GUILD_ID`, os comandos são registrados imediatamente no servidor de desenvolvimento. Sem `GUILD_ID`, o registro é global e pode demorar para aparecer no Discord.

## Variáveis de ambiente

| Variável | Obrigatória | Descrição |
| --- | --- | --- |
| `DISCORD_TOKEN` | Sim | Token do bot. `TOKEN` também é aceito. |
| `CLIENT_ID` | Para deploy | ID da aplicação Discord. |
| `GUILD_ID` | Não | Servidor usado para registro rápido dos comandos. |
| `DEFAULT_PREFIX` | Não | Prefixo legado; padrão `!`. |
| `ENABLE_PREFIX_COMMANDS` | Não | Habilita comandos por prefixo; padrão `false`. |
| `ENABLE_AUTOMOD` | Não | Habilita automod baseado em mensagens; padrão `false`. |
| `ENABLE_LEVELS` | Não | Habilita XP por mensagens; padrão `false`. |
| `ENABLE_MEMBER_EVENTS` | Não | Habilita eventos de entrada e saída de membros. |
| `MONGO_URI` | Não inicialmente | URI do MongoDB Atlas. Necessária para economia, níveis e configurações persistentes. |
| `REQUIRE_DATABASE` | Não | Se `true`, impede o login quando o MongoDB não conecta. |

## Intents e permissões

O bot sempre usa `Guilds`. Os demais intents são ativados somente quando necessários:

- `Message Content Intent`: necessário para `ENABLE_PREFIX_COMMANDS`, `ENABLE_AUTOMOD` ou `ENABLE_LEVELS`.
- `Server Members Intent`: necessário para `ENABLE_MEMBER_EVENTS`.

Os intents privilegiados também precisam ser habilitados no Developer Portal. O bot precisa receber no servidor as permissões usadas por cada módulo, como `Manage Messages`, `Moderate Members`, `Ban Members`, `Manage Channels`, `Manage Guild` e `Administrator` quando aplicável.

## Comandos

### Informação

- `/help [comando]`: Help por categoria com Components V2.
- `/ping`: latência do bot.
- `/avatar [usuario]`: avatar de um usuário.
- `/userinfo [usuario]`: informações de um usuário.
- `/serverinfo`: informações do servidor.

### Utilitários

- `/calc expressao`: calcula expressões matemáticas com segurança.
- `/devex robux moeda`: calcula o valor estimado de Robux no Developer Exchange.
- `/resume url`: resume o conteúdo de um site usando o Jina Reader, com limite de 3 usos por pessoa a cada minuto.
- `/verify [canal]`: publica o painel que entrega o cargo `Membro`.
- `/tech [canal]`: publica um painel com 25 botões de linguagens e tecnologias.
- `/cores [canal]`: publica o painel com os cargos `Vermelho`, `Branco`, `Preto`, `Vermelho Vinho`, `Rosa`, `Amarelo`, `Verde`, `Verde Escuro`, `Azul`, `Roxo`, `Laranja` e `Marrom`.
- `/novidades [canal] [titulo] [descricao]`: publica um embed roxo de novidades.
- `/regras enviar|adicionar|editar|remover|config|listar`: gerencia o embed persistente de regras.

Exemplo:

```text
/calc expressao:(10 + 5) * 2
```

Suporta números decimais, `+`, `-`, `*`, `/`, `%`, `^`, sinais unários e parênteses. A expressão é interpretada por um parser próprio; o bot não usa `eval`.

Exemplo:

```text
/devex robux:30000 moeda:BRL
```

O comando usa a taxa DevEx nova de `US$ 0,0038 por Robux` e converte o valor em USD para a moeda escolhida. O campo `moeda` tem autocomplete por código ISO 4217 e nome; são aceitas as moedas cobertas pelo serviço de câmbio. As cotações são referências diárias e o resultado não considera impostos, taxas ou retenções.

### Convivência

- `/afk`: define ou remove status de ausência.
- `/fun`: ações sociais e diversão.
- `/poll`: cria enquete interativa.
- `/remind`: cria lembrete persistente.

### Jogos

- `/blackjack`: blackjack com carteira e botões protegidos pelo jogador.
- `/coinflip`: aposta em cara ou coroa.

### Economia

- `/balance [usuario]`: carteira, banco e total.
- `/daily`: recompensa diária.
- `/work`: recompensa por trabalho.
- `/pay usuario valor`: transferência entre usuários.
- `/shop`: loja.
- `/inventory [usuario]`: inventário.
- `/transactions`: extrato.

### Níveis

- `/level [usuario]`: nível e XP.
- `/leaderboard [periodo]`: ranking em embed.
- `/ranking [periodo]`: ranking visual em canvas.
- `/profile [usuario]`: perfil visual em canvas.
- `/level-config`: configura XP e anúncios.

### Moderação e configuração

- `/warn`, `/warnings`, `/timeout`, `/untimeout`.
- `/kick`, `/ban`, `/unban`.
- `/clear`, `/lock`, `/unlock`.
- `/automod config|palavra|dominio|whitelist|log`.
- `/welcome config|disable|test|color-role|panel`.

Os comandos de moderação, automod, boas-vindas e `/level-config` são administrativos. Eles recebem `default_member_permissions: Administrator`, são ocultados do Help de membros comuns e possuem uma segunda checagem no executor de slash/prefixo.

## Calculadora DevEx

Arquivos principais:

- `src/commands/utilities/devex.ts`: slash command, autocomplete e resposta.
- `src/services/DevexService.ts`: taxa DevEx, catálogo de moedas, cache e conversão.

Constante atual:

```ts
DEVEX_USD_PER_ROBUX = 0.0038
```

O catálogo e as cotações usam a API pública Frankfurter v2, sem chave de API. O catálogo fica em cache por 24 horas e cada cotação por 15 minutos. Se o catálogo estiver temporariamente indisponível, o autocomplete usa uma lista local de fallback; se a cotação não puder ser obtida, o comando responde com erro efêmero.

### Resumo de sites

```text
/resume url:https://example.com/artigo
```

O comando consulta o [Jina Reader](https://jina.ai/en-US/reader/) e usa o campo `data.content` retornado pela API. Esse campo é uma string Markdown; o bot preserva títulos, listas, links e quebras de linha ao colocá-lo no embed. Não é executado nenhum modelo de IA local ou externo. Cada usuário pode executar até 3 leituras por janela de 60 segundos; esse contador fica em memória e é reiniciado quando o processo do bot é reiniciado.

### Painéis administrativos

Os painéis são publicados diretamente no canal escolhido; o comando responde apenas com uma confirmação efêmera. Os comandos usam Components V2 quando precisam manter botões ou select menus dentro do mesmo container visual.

- `/verify`: exige um cargo chamado `Membro`. O botão verifica o usuário e entrega esse cargo.
- `/cores`: exige os 12 cargos de cor com os nomes exatos. O usuário pode escolher uma cor e a anterior é removida.
- `/tech`: publica 25 botões em cinco linhas. Cada botão responde de forma efêmera com uma descrição da tecnologia.
- `/novidades`: publica um embed de atualizações com título e descrição configuráveis.
- `/regras enviar canal:<canal>`: envia o prefab de regras para o canal e guarda `channelId`/`messageId`.
- `/regras adicionar titulo:<titulo> texto:<texto>`: adiciona uma regra com o próximo ID livre.
- `/regras editar id:<id> titulo:<novo> texto:<novo>`: altera um ou ambos os campos da regra.
- `/regras remover id:<id|all>`: remove uma regra ou todas.
- `/regras config`: altera título, descrição e cor hexadecimal do embed.
- `/regras listar`: mostra os IDs e os textos atuais de forma efêmera.

Alterações em regras sincronizam automaticamente o último embed publicado, quando ele ainda existe. Os comandos `verify`, `tech`, `cores` e `regras` são restritos a administradores.

## Help com Components V2

O Discord não aceita um select menu dentro de uma embed tradicional. O Help usa o modelo equivalente dos Components V2:

```text
Container (17)
├── Text Display (10)
└── Action Row (1)
    └── String Select (3)
```

As mensagens usam `MessageFlags.IsComponentsV2`. O `custom_id` do menu contém o ID do autor (`help:category:<userId>`), e o handler rejeita interações de outras pessoas.

## Arquitetura

```text
LarperBot/
├── index.ts                    # Entrada do bot
├── src/
│   ├── commands/               # Slash commands e adaptadores legados
│   ├── events/                 # Eventos do gateway
│   ├── models/                 # Schemas Mongoose
│   ├── services/               # Regras de negócio e integrações
│   ├── structures/             # CustomClient, Commands, Event e Logger
│   └── utils/                  # Banco, moderação e perfis
├── docs/ROADMAP.md             # Roadmap detalhado
├── .env.example                # Modelo de configuração
├── package.json                # Scripts e dependências
└── tsconfig.json               # Configuração TypeScript
```

### Padrão de um comando

1. Criar um arquivo em `src/commands/<categoria>/`.
2. Estender `Commands`.
3. Definir `name`, `description`, `category` e `data` com `SlashCommandBuilder`.
4. Implementar `executeInteraction`.
5. Implementar `executeAutocomplete` quando houver opção com `.setAutocomplete(true)`.
6. Adicionar o comando ao README e ao roadmap quando for uma nova área.
7. Executar build e deploy dos comandos.

O `CustomClient` carrega automaticamente arquivos `.ts` e `.js` dentro de `src/commands` e `src/events`.

## MongoDB

O MongoDB é opcional na inicialização quando `REQUIRE_DATABASE=false`. Sem conexão, comandos que exigem persistência respondem informando que o banco está indisponível.

Para o MongoDB Atlas:

1. Crie usuário e cluster.
2. Adicione o IP atual em Network Access.
3. Copie a connection string para `MONGO_URI`.
4. Reinicie o bot.

As opções do Mongoose usam `returnDocument: 'after'`, evitando o aviso de depreciação de `new: true`.

## Roadmap

O roadmap detalhado está em [`docs/ROADMAP.md`](docs/ROADMAP.md). O MVP das áreas principais já está implementado. O trabalho restante é principalmente hardening: testes automatizados, cooldowns uniformes, retries, índices, auditoria, health check, métricas e graceful shutdown.

## Troubleshooting

### MongoDB Atlas não conecta

Verifique `MONGO_URI`, usuário, senha, IP liberado no Atlas e se o cluster está ativo. Durante o desenvolvimento, mantenha `REQUIRE_DATABASE=false` se quiser que o bot inicie sem persistência.

### `Used disallowed intents`

Desative os recursos que exigem intents privilegiados ou habilite os intents correspondentes no Developer Portal e no `.env`.

### Slash commands não aparecem

Confirme `CLIENT_ID`, `GUILD_ID` e token, execute `corepack pnpm deploy:commands` e reinicie o Discord. Comandos globais podem levar tempo para propagar.

### O bot parece travado no `pnpm dev`

`tsx watch index.ts` permanece executando para observar alterações. Isso é esperado. Um bot iniciado com sucesso deve exibir as mensagens de conexão com MongoDB/Discord.

## Checklist para novas sessões

1. Ler este README e `docs/ROADMAP.md`.
2. Verificar o estado do worktree antes de editar.
3. Preservar slash commands como interface principal.
4. Não adicionar intents sem necessidade.
5. Proteger ações administrativas com permissões e validação de hierarquia.
6. Usar `MessageFlags.Ephemeral`, nunca `ephemeral: true`.
7. Usar `clientReady`, não o evento legado `ready`.
8. Rodar `corepack pnpm build` após alterações.
9. Rodar `corepack pnpm deploy:commands` quando o schema de um slash command mudar.
10. Não colocar tokens, connection strings ou arquivos `.env` no commit.
