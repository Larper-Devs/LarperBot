# Roadmap do LarperBot

## Direção técnica

O bot será organizado por módulos de domínio, com serviços independentes das interfaces de comando. Prefix commands continuam disponíveis para compatibilidade, mas novas funcionalidades devem priorizar slash commands e componentes interativos do Discord.

## Status da entrega

Esta versão entrega um MVP funcional das fases de produto. O núcleo de operação e os comandos estão implementados; a fase de qualidade continua como hardening antes de produção.

| Fase | Status |
| --- | --- |
| Fundação e migração | Concluída |
| Moderação | MVP concluído: `warn`, `warnings`, `timeout`, `untimeout`, `kick`, `ban`, `unban`, `clear`, `lock` e `unlock` |
| Moderação automática | MVP concluído: spam, repetição, menções, caps, palavras, domínios e whitelist |
| Convivência | MVP concluído: informação, enquete, lembrete, AFK, 8ball, ship e interações sociais |
| Minigames | MVP concluído: blackjack interativo, coinflip e componentes protegidos por jogador |
| Economia | MVP concluído: carteira, daily, work, pay, loja, inventário e extrato |
| Níveis | MVP concluído: XP total, semanal e mensal, rankings, configuração, perfil e cards gerados em canvas |
| Boas-vindas | MVP concluído: configuração, teste, auto role e painel de cores por select menu |
| Qualidade e operação | Em andamento: testes automatizados, índices/retries avançados e rollout |

Os módulos que recebem mensagens ou eventos de membros são opt-in por ambiente: `ENABLE_AUTOMOD`, `ENABLE_LEVELS` e `ENABLE_MEMBER_EVENTS`. Slash commands continuam sendo o caminho padrão e exigem apenas o intent `Guilds`.

Estrutura alvo:

```text
src/
  commands/       # slash commands e adaptadores de prefixo
  events/         # gateway events e interação
  modules/        # moderação, economia, níveis, boas-vindas etc.
  models/         # schemas Mongoose
  services/       # regras de negócio e tarefas agendadas
  components/     # botões, menus e modais
  utils/          # permissões, cooldowns, embeds e validações
```

Regras transversais: toda ação administrativa deve validar hierarquia e permissões; toda recompensa deve ser idempotente; toda configuração deve ser por servidor; e ações relevantes devem gerar log de auditoria.

## Fase 0 — Migração e fundação

**Entregue nesta migração**

- substituir `stoat.js` por `discord.js`;
- manter TypeScript e carregamento automático de arquivos;
- suportar `help` e `clear` por prefixo e slash command;
- criar o deploy de comandos em `src/deploy-commands.ts`;
- adicionar intents para guilds, mensagens, conteúdo e membros;
- tornar MongoDB opcional durante a inicialização;
- separar configuração de ambiente, logs e tratamento de erros.

**Próximo hardening**

- adicionar testes unitários para permissões, parsing e serviços;
- centralizar respostas em embeds e mensagens localizadas;
- adicionar cooldown por usuário, comando e servidor;
- criar sistema de erro com código público e stack trace apenas no log;
- adicionar health check, métricas de comandos e graceful shutdown.

## Fase 1 — Sistema de moderação

Comandos iniciais: `/warn`, `/warnings`, `/clear`, `/timeout`, `/untimeout`, `/kick`, `/ban`, `/unban`, `/softban`, `/lock` e `/unlock`.

Entrega:

- casos de moderação com ID, autor, alvo, motivo, duração, data e status;
- `warn` persistente, consulta, remoção e histórico;
- timeout com expiração automática e conversão segura de duração;
- kick, ban e unban com verificação de hierarquia;
- proteção contra moderar o próprio bot, o dono do servidor ou membro acima do bot;
- canal de logs configurável com embeds e referência ao caso;
- mensagens efêmeras para ações administrativas sensíveis;
- permissões configuráveis por comando sem confiar apenas no nome do cargo.

Critério de aceite: nenhuma ação é executada sem permissão, toda falha informa a causa ao moderador e cada ação bem-sucedida pode ser auditada.

## Fase 2 — Moderação automática

Primeira versão:

- anti-spam por janela deslizante;
- limite de mensagens repetidas;
- proteção contra mention spam;
- filtro de links, convites e palavras configuráveis;
- limite de caps lock e caracteres especiais;
- whitelist de canais, cargos, domínios e membros;
- níveis de ação: apagar, avisar, timeout e encaminhar para revisão;
- score de infrações por usuário com decaimento;
- logs de cada detecção, regra aplicada e ação tomada.

O módulo deve combinar AutoMod nativo do Discord quando atender ao caso com regras de aplicação para o que exigir contexto. Regras configuráveis não podem ficar hardcoded em listeners.

Critério de aceite: falsos positivos podem ser revertidos por whitelist, cada regra tem cooldown e o sistema não cria uma cascata de punições para a mesma mensagem.

## Fase 3 — Comandos de convivência

Comandos sugeridos: `/ping`, `/avatar`, `/banner`, `/userinfo`, `/serverinfo`, `/servericon`, `/roleinfo`, `/botinfo`, `/invite`, `/say`, `/poll`, `/remind`, `/afk`, `/ship`, `/hug`, `/kiss`, `/pat`, `/cuddle` e `/8ball`.

Entrega:

- respostas rápidas de informação do usuário, servidor e bot;
- ações sociais sem abuso, com cooldown e opção de desativação por servidor;
- enquetes com botões ou reações e encerramento automático;
- lembretes persistentes e reativados após reinício;
- mensagens configuráveis sem permitir menções abusivas por padrão;
- tratamento uniforme de usuário ausente, canal incompatível e permissões.

## Fase 4 — Minigames

Ordem recomendada: blackjack, coinflip, dados, maior-menor, roleta e jogos de reação.

Blackjack deve usar uma sessão por usuário/canal, estado persistente durante a partida, botões `hit`, `stand` e `double`, timeout de inatividade e encerramento idempotente. O sorteio deve usar `crypto.randomInt`, nunca `Math.random` para apostas.

Base comum dos jogos:

- aposta mínima e máxima configuráveis;
- bloqueio de duas partidas conflitantes;
- resultado com histórico e auditoria;
- componentes com `customId` versionado;
- autorização do jogador antes de aceitar um botão;
- integração com a carteira da economia somente após a transação ser confirmada.

## Fase 5 — Economia própria

Modelo inicial por servidor:

- carteira e banco separados;
- saldo, depósito, saque, transferência e extrato;
- `/daily`, `/work`, `/crime` ou atividades equivalentes com cooldown;
- loja, itens, inventário e uso de itens;
- recompensas de jogos e penalidade de apostas;
- imposto ou taxa de transferência opcional;
- transações com valor, origem, destino, motivo, idempotency key e timestamp.

Regras de segurança:

- nunca alterar saldo com operações isoladas sem registro;
- usar transação do MongoDB quando disponível;
- impedir saldo negativo salvo nos débitos explicitamente permitidos;
- limitar transferências e detectar auto-transferência circular;
- separar moeda de cada servidor e não confiar em valores vindos do cliente.

## Fase 6 — Sistema de level mensal, semanal e total

O sistema terá três rankings independentes:

- **total:** XP acumulado desde a entrada do usuário no sistema;
- **semanal:** XP da semana ISO atual, com fechamento e ranking;
- **mensal:** XP do mês atual, com fechamento e ranking.

Entrega:

- XP por mensagem com cooldown, limite diário e canais ignorados;
- XP por voz apenas com presença real e regras anti-AFK;
- `/level`, `/rank`, `/leaderboard` e `/level-config`;
- recompensas por patamar e anúncio configurável;
- snapshots de fechamento para manter o histórico;
- tarefas agendadas resilientes a restart e timezone configurável;
- remoção de XP e correções administrativas auditadas.

O ganho deve ser calculado no servidor e persistido de maneira idempotente. Mensagens do bot, spam e conteúdo repetido não devem gerar XP.

## Fase 7 — Boas-vindas e onboarding

Configuração sugerida por servidor: `/welcome config`, `/welcome test`, `/welcome disable` e `/welcome panel`.

Recursos:

- canal de entrada e canal de saída;
- mensagem simples ou embed com placeholders como `{user}`, `{server}`, `{memberCount}` e `{mention}`;
- imagem, thumbnail, cor, footer e timestamp configuráveis;
- cargo automático com validação de hierarquia;
- painel de escolha de cores por select menu;
- painel de regras/autoroles com botões e select menus;
- respostas efêmeras para configuração;
- preview antes de salvar;
- persistência dos componentes e reconstrução após reinício;
- IDs de componentes com servidor, versão e ação, sem guardar estado crítico apenas na memória.

Critério de aceite: uma configuração incompleta não é salva, o bot não menciona `@everyone` sem opt-in explícito e componentes antigos falham de forma amigável após uma atualização.

## Fase 8 — Qualidade e operação

- testes unitários dos serviços de economia, XP e moderação;
- testes de integração com MongoDB de teste;
- mocks de interações, botões, menus e permissões;
- validação de comandos em servidor de desenvolvimento antes do deploy global;
- backups e índice TTL para timeouts, lembretes e casos expirados;
- rate limit interno, filas para tarefas e retry limitado;
- logs estruturados sem token, conteúdo sensível ou dados desnecessários;
- documentação de permissões e checklist de lançamento.

## Ordem de execução recomendada

1. completar hardening da Fase 0;
2. implementar casos e comandos de moderação;
3. adicionar automod e logs;
4. criar configurações por servidor;
5. implementar economia e transações;
6. adicionar níveis e snapshots semanais/mensais;
7. lançar comandos de convivência;
8. integrar minigames à economia;
9. finalizar boas-vindas, painéis e autoroles;
10. executar testes de carga, revisão de permissões e rollout gradual.

## Comandos previstos por módulo

| Módulo | Comandos principais |
| --- | --- |
| Moderação | `warn`, `warnings`, `timeout`, `untimeout`, `kick`, `ban`, `unban`, `clear`, `lock`, `unlock` |
| Automod | `automod config`, `automod palavra`, `automod dominio`, `automod whitelist`, `automod log` |
| Convivência | `ping`, `avatar`, `userinfo`, `serverinfo`, `poll`, `remind`, `afk`, `fun` |
| Jogos | `blackjack`, `coinflip` |
| Economia | `balance`, `daily`, `work`, `pay`, `shop`, `inventory`, `transactions` |
| Níveis | `level`, `leaderboard`, `ranking`, `profile`, `level-config` |
| Boas-vindas | `welcome config`, `welcome test`, `welcome panel`, `welcome disable`, `welcome color-role` |
