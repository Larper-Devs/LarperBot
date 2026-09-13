import { readdirSync } from "fs";
import { join } from "path";
import { fileURLToPath, pathToFileURL } from "url";
import { connect } from "mongoose";
import {
  Client,
  ClientOptions,
  GatewayIntentBits,
} from "discord.js";
import { Logger } from "./Logger.js";
import { Commands } from "./Commands.js";
import { startReminderScheduler } from "../services/ReminderService.js";

class CustomClient extends Client {
  readonly commands = new Map<string, Commands>();
  readonly botToken: string;
  readonly prefix: string;
  readonly prefixCommandsEnabled: boolean;
  readonly automodEnabled: boolean;
  readonly levelsEnabled: boolean;
  readonly memberEventsEnabled: boolean;
  readonly logger: Logger;
  databaseReady = false;
  private eventsLoaded = false;

  constructor(options?: ClientOptions) {
    const enablePrefixCommands =
      process.env.ENABLE_PREFIX_COMMANDS?.toLowerCase() === "true";
    const enableAutomod =
      process.env.ENABLE_AUTOMOD?.toLowerCase() === "true";
    const enableLevels =
      process.env.ENABLE_LEVELS?.toLowerCase() === "true";
    const enableMemberEvents =
      process.env.ENABLE_MEMBER_EVENTS?.toLowerCase() === "true";
    const enableMessageFeatures = enablePrefixCommands || enableAutomod || enableLevels;
    const intents: GatewayIntentBits[] = [GatewayIntentBits.Guilds];

    if (enableMessageFeatures) {
      intents.push(
        GatewayIntentBits.GuildMessages,
        GatewayIntentBits.MessageContent,
      );
    }

    if (enableMemberEvents) {
      intents.push(GatewayIntentBits.GuildMembers);
    }

    const defaultOptions: ClientOptions = {
      intents,
    };

    super({
      ...defaultOptions,
      ...options,
      intents: options?.intents ?? defaultOptions.intents,
    });

    this.botToken = process.env.DISCORD_TOKEN ?? process.env.TOKEN ?? "";
    this.prefix = process.env.DEFAULT_PREFIX?.trim() || "!";
    this.prefixCommandsEnabled = enablePrefixCommands;
    this.automodEnabled = enableAutomod;
    this.levelsEnabled = enableLevels;
    this.memberEventsEnabled = enableMemberEvents;
    this.logger = new Logger();
  }

  async start(): Promise<void> {
    if (!this.botToken) {
      throw new Error("Defina DISCORD_TOKEN (ou TOKEN) no arquivo .env.");
    }

    await this.loadEvents();
    await this.loadCommands();

    const mongoUri = process.env.MONGO_URI?.trim();
    if (mongoUri) {
      try {
        this.logger.info("Conectando ao MongoDB...");
        await connect(mongoUri, { serverSelectionTimeoutMS: 10_000 });
        this.databaseReady = true;
        this.logger.success("Banco de dados conectado.");
      } catch (error) {
        this.logger.error("Não foi possível conectar ao MongoDB", error);

        if (process.env.REQUIRE_DATABASE?.toLowerCase() === "true") {
          throw error;
        }

        this.logger.warn(
          "Continuando sem persistência. Corrija o acesso ao MongoDB para ativar economia, níveis e configurações.",
        );
      }
    } else {
      this.logger.warn(
        "MONGO_URI não configurada; o bot iniciou sem persistência.",
      );
    }

    this.logger.info("Conectando ao Discord...");
    await this.login(this.botToken);
    startReminderScheduler(this);
  }

  private getRuntimeSourcePath(...parts: string[]): string {
    const runtimeRoot = fileURLToPath(new URL("../", import.meta.url));
    return join(runtimeRoot, ...parts);
  }

  async loadEvents(): Promise<void> {
    if (this.eventsLoaded) return;

    const eventsRoot = this.getRuntimeSourcePath("events");
    const categories = readdirSync(eventsRoot, { withFileTypes: true });

    for (const category of categories) {
      if (!category.isDirectory()) continue;
      const categoryPath = join(eventsRoot, category.name);
      const events = readdirSync(categoryPath, { withFileTypes: true });

      for (const event of events) {
        if (
          !event.isFile() ||
          (!event.name.endsWith(".ts") && !event.name.endsWith(".js"))
        ) {
          continue;
        }

        const eventPath = pathToFileURL(join(categoryPath, event.name)).href;
        const eventModule = await import(eventPath);
        const EventClass = eventModule.default || eventModule;
        const loadedEvent = new EventClass(this);

        this.on(loadedEvent.name, loadedEvent.run.bind(loadedEvent));
      }
    }

    this.eventsLoaded = true;
  }

  async loadCommands(): Promise<void> {
    if (this.commands.size > 0) return;

    const commandsRoot = this.getRuntimeSourcePath("commands");
    const categories = readdirSync(commandsRoot, { withFileTypes: true });

    for (const category of categories) {
      if (!category.isDirectory()) continue;
      const categoryPath = join(commandsRoot, category.name);
      const commands = readdirSync(categoryPath, { withFileTypes: true });

      for (const command of commands) {
        if (
          !command.isFile() ||
          (!command.name.endsWith(".ts") && !command.name.endsWith(".js"))
        ) {
          continue;
        }

        const commandPath = pathToFileURL(
          join(categoryPath, command.name),
        ).href;
        const commandModule = await import(commandPath);
        const CommandClass = commandModule.default || commandModule;
        const loadedCommand = new CommandClass(this) as Commands;

        this.commands.set(loadedCommand.name, loadedCommand);
      }
    }
  }

  get commandList(): Commands[] {
    return [...this.commands.values()];
  }

  findCommand(name: string): Commands | undefined {
    const query = name.toLowerCase();
    return this.commandList.find(
      (command) =>
        command.name.toLowerCase() === query ||
        command.aliases.some((alias) => alias.toLowerCase() === query),
    );
  }
}

export { CustomClient };
