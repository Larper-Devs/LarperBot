# LarperBot em Rust

A versão Rust fica em [`src/main.rs`](src/main.rs) e usa Serenity, SQLite local, TLS via Rustls e renderização nativa de cards PNG para o perfil.

## Instalar Rust no Windows

PowerShell (instalador oficial):

```powershell
irm https://win.rustup.rs -UseBasicParsing | iex
```

Aceite os padrões do instalador e abra um novo PowerShell. Verifique:

```powershell
rustc --version
cargo --version
rustup default stable
rustup update
```

Se o compilador reclamar de linker no Windows, instale o **Visual Studio Build Tools** com o workload **Desktop development with C++** e execute o build novamente.

## Instalar Rust no Linux

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version
cargo --version
rustup default stable
rustup update
```

Em Debian/Ubuntu, instale as ferramentas de compilação:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev
```

## Rodar localmente

```powershell
Copy-Item .env.example .env
$env:DISCORD_TOKEN = "seu_token"
cargo fmt -- --check
cargo check
cargo run
```

O bot aceita `DISCORD_TOKEN` ou `TOKEN`. `GUILD_ID` registra os comandos imediatamente nesse servidor; sem ele, o registro é global. As variáveis `ENABLE_PREFIX_COMMANDS`, `ENABLE_AUTOMOD`, `ENABLE_LEVELS` e `ENABLE_MEMBER_EVENTS` continuam funcionando. O XP fica ativo por padrão quando `ENABLE_LEVELS` não é definido e pode ser desligado por servidor com `/level-config ativado:false`. Com `ENABLE_MEMBER_EVENTS=true`, o cargo definido em `UNVERIFIED_ROLE_ID` (por padrão `918519844020830236`) é atribuído automaticamente a novos membros. O bot precisa da permissão **Manage Roles**, e o cargo do bot deve estar acima desse cargo na hierarquia do servidor. O banco local é `./larperbot.sqlite` e pode ser alterado com `DATABASE_PATH`.

Quando `MONGO_URI` estiver definido, o SQLite local continua sendo o cache persistente consultado primariamente pelas funcionalidades do bot. No start, os dados da colecao `rust_cache` sao carregados para o SQLite; a antiga colecao `userprofiles` tambem e importada para compatibilidade com a versao Node. Depois da carga inicial, o snapshot local e enviado ao MongoDB a cada 30 minutos, inclusive para `userprofiles`. `MONGO_DATABASE` e opcional: se nao for definido, o banco presente na URI e usado; sem banco na URI, o padrao e `larperbot`. Se o MongoDB estiver indisponivel, o bot continua usando o cache em disco e tenta sincronizar no proximo ciclo.

## Render

O processo também mantém um servidor HTTP leve com as rotas `/`, `/health` e `/healthz`. Ele escuta em `0.0.0.0:$PORT`, que é a porta fornecida pelo Render, e roda em paralelo ao bot do Discord. Se `PORT` não existir, a execução local usa `10000`.

Configure o serviço do Render como **Web Service** e use, por exemplo:

```text
Build Command: cargo build --release
Start Command: ./target/release/larperbot
Health Check Path: /healthz
```

Não é necessário simular requisições internamente: o endpoint permite que o Render verifique o serviço. Em planos com suspensão por inatividade, um ping interno não garante disponibilidade contínua; para uptime permanente, é necessário um plano que não suspenda o serviço ou um monitor externo autorizado.

## Build de produção

```bash
cargo build --release
```

O executável gerado é `target/release/larperbot` no Linux e `target/release/larperbot.exe` no Windows. O perfil release usa `opt-level=z`, LTO, `panic=abort` e strip; no ambiente de desenvolvimento deste projeto o binário Windows mediu aproximadamente 6,9 MB.

## Discloud

O [`discloud.config`](discloud.config) já aponta para Rust:

```text
MAIN=target/release/larperbot
BUILD=cargo build --release
START=./target/release/larperbot
RAM=100
```

Envie `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `discloud.config` e os arquivos necessários do projeto. Não envie `.env`, tokens ou o diretório `target`; a Discloud recompila usando `BUILD`. Cadastre as variáveis de ambiente no painel da Discloud, principalmente `DISCORD_TOKEN`, e ative no Developer Portal apenas os intents usados.

O limite `RAM=100` é o limite configurado para o app, não uma garantia matemática de consumo em todos os servidores. O código reduz o uso mantendo somente intents opcionais, sem cache completo do Discord e com SQLite. Para vários processos ou persistência fora do disco da Discloud, use um banco externo e substitua a camada `Db`.

## Diferenças da migração

Os comandos e eventos principais foram mantidos: informação, utilidades, painéis, feed, regras, economia, níveis, jogos, automod, boas-vindas, lembretes e moderação. `/addsaldo usuario valor [motivo]` é exclusivo para administradores, credita a carteira e registra a operação no extrato. `profile`, `ranking` e `leaderboard` respondem com cards PNG; o ranking mostra os cinco primeiros com posição, avatar, username e XP total. `/profile-config tema` permite escolher entre vários temas. O SQLite é o cache local primário, e o MongoDB configurado em `MONGO_URI` é a persistência remota sincronizada.

## Components V2

A auditoria da versão Node encontrou quatro fluxos que usavam `MessageFlags.IsComponentsV2`: `/help`, `/verify`, `/tech` e `/cores`. Todos agora enviam um `Container` roxo com `TextDisplay` e os componentes interativos equivalentes no Rust. As interações dos painéis usam os mesmos identificadores da versão Node (`panel:verify:<roleId>`, `panel:tech` e `panel:colors`).

O Serenity 0.12.5 ainda não oferece builders para os componentes V2. Por isso, esses payloads são enviados como JSON pela camada HTTP do próprio Serenity, mantendo autenticação e rate limit do cliente. Embeds tradicionais não são combinados com `IsComponentsV2`, pois o Discord não aceita os dois formatos na mesma mensagem.

O painel de cores de `/welcome panel` permanece com embed tradicional porque a versão Node também não o publicava como Components V2. `/ticket` é um painel adicional da versão Rust e não possui equivalente no commit Node auditado.

Para usar os painéis de cores e tecnologias, o usuário precisa possuir o cargo verificado de ID `918519844020830235`. Os fluxos `/verify` e o botão legado de verificação entregam esse mesmo cargo; usuários sem ele recebem uma resposta privada e nenhum cargo é alterado. O `/tech` usa um menu de seleção múltipla e referencia os IDs de cargos de `cargos.md`; ao confirmar, cargos desmarcados são removidos e cargos selecionados são adicionados.

## Ícones do painel `/tech` e cargos de level

Ao publicar `/tech`, o bot consulta os emojis personalizados do servidor e adiciona à opção o emoji cujo nome corresponde à tecnologia (`javascript`, `typescript`, `rust`, `csharp`, `vuejs`, `nodejs`, `nextjs`, `react`, `mongodb` e outros aliases). Se o emoji não existir ou estiver indisponível, a opção continua funcionando sem ícone.

Ao subir de level, o bot publica um embed somente com a descrição `<@usuario> agora é level X!` no canal configurado. Se existir um cargo chamado `Level X`, `Nível X`, `Nivel X` ou `Lvl X` e o bot conseguir entregá-lo, a descrição passa a ser `Parabéns <@usuario>! Agora você é level X e ganhou <@&cargo>.`.

`/captcha` publica o aviso bilíngue diretamente no canal `#captcha` de ID `918519844591255613` e ativa a proteção anti-spam nesse canal. Qualquer mensagem enviada por usuário, bot ou webhook nesse canal causa banimento com remoção das mensagens recentes; a mensagem publicada pelo próprio LarperBot é ignorada. O bot precisa da permissão `Banir membros`.

## Logs de punição

Toda punição aplicada pelo bot ou por comando manual é registrada em um embed no canal de ID `918519846461923329`. Os registros incluem ação, resultado, servidor, horário, alvo, executor, motivo, canal de origem e detalhes adicionais. Isso cobre advertência, expulsão, banimento, remoção de banimento, timeout, remoção de timeout, limpeza de mensagens, bloqueio/desbloqueio de canal, remoções do automod e banimentos automáticos do `#captcha`.

Para punições feitas diretamente pela interface do Discord, o bot acompanha o evento de auditoria do servidor e registra banimentos, expulsões, timeouts e remoções de mensagens com o executor e o motivo informados no Audit Log. O bot precisa do intent `GUILD_MODERATION` e da permissão `Ver registro de auditoria`.
