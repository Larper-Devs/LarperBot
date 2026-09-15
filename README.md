# LarperBot em Rust

A versão Rust fica em [`src/main.rs`](src/main.rs) e usa Serenity, SQLite local e TLS via Rustls. O binário release é enxuto e não inclui Node.js, pnpm, Mongoose ou canvas.

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

O bot aceita `DISCORD_TOKEN` ou `TOKEN`. `GUILD_ID` registra os comandos imediatamente nesse servidor; sem ele, o registro é global. As variáveis `ENABLE_PREFIX_COMMANDS`, `ENABLE_AUTOMOD`, `ENABLE_LEVELS` e `ENABLE_MEMBER_EVENTS` continuam funcionando. O banco local é `./larperbot.sqlite` e pode ser alterado com `DATABASE_PATH`.

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

O limite `RAM=100` é o limite configurado para o app, não uma garantia matemática de consumo em todos os servidores. O código reduz o uso mantendo somente intents opcionais, sem cache completo do Discord, sem canvas e com SQLite. Para vários processos ou persistência fora do disco da Discloud, use um banco externo e substitua a camada `Db`.

## Diferenças da migração

Os comandos e eventos principais foram mantidos: informação, utilidades, painéis, feed, regras, economia, níveis, jogos, automod, boas-vindas, lembretes e moderação. `/addsaldo usuario valor [motivo]` é exclusivo para administradores, credita a carteira e registra a operação no extrato. `profile` e `ranking` usam embeds textuais em vez de imagens canvas, preservando os dados sem carregar uma biblioteca gráfica pesada. O SQLite substitui o MongoDB; dados existentes do MongoDB precisam ser exportados/importados separadamente.
