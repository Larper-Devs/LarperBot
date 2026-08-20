import { readdirSync } from "fs";
import { join } from "path";
import { pathToFileURL } from "url";
import { connect } from "mongoose";
import { Client, Message, ClientOptions } from "stoat.js";
import { Logger } from "./Logger";

interface Command {
  name: string;
  description: string;
  category: string;
  aliases: string[];
  howToUse: string;
  logger: Logger;
  run(client: CustomClient, message: Message, args: string[]): Promise<any>;
}

class CustomClient extends Client {
  commands: Command[];
  token: string;
  logger: Logger;

  constructor(token: string, options?: ClientOptions) {
    super(options);
    this.commands = [];
    this.token = token;
    this.logger = new Logger();
    this.init();
  }
  
  private async init() {
    await this.loadEvents();
    await this.loadCommands();
    await this.loginBot(this.token);
    
    await connect(`${process.env.MONGO_URI}`);
    this.logger.success('DB Connected!');
  }

  async loadEvents() {
    const categories = readdirSync("src/events");

    for (const category of categories) {
      const events = readdirSync(`src/events/${category}`);

      for (const event of events) {
        if (!event.endsWith(".ts") && !event.endsWith(".js")) continue;
        const eventPath = pathToFileURL(
          join(process.cwd(), `src/events/${category}/${event}`),
        ).href;
        const eventModule = await import(eventPath);
        const EventClass = eventModule.default || eventModule;
        const evt = new EventClass(this);

        this.on(evt.name, evt.run);
      }
    }
  }

  async loadCommands() {
    const categories = readdirSync("src/commands");

    for (const category of categories) {
      const commands = readdirSync(`src/commands/${category}`);

      for (const command of commands) {
        if (!command.endsWith(".ts") && !command.endsWith(".js")) continue;
        const commandPath = pathToFileURL(
          join(process.cwd(), `src/commands/${category}/${command}`),
        ).href;
        const commandModule = await import(commandPath);
        const CommandClass = commandModule.default || commandModule;
        const cmd = new CommandClass(this);

        this.commands.push(cmd);
      }
    }
  }

}

export { Command, CustomClient };
