import { readdirSync } from "fs";
import { join } from "path";
import { pathToFileURL } from "url";

import { Client, Message, ClientOptions } from 'stoat.js';

interface iOfNormal {
    name: string;
    description: string;
    category: string;
    aliases: string[];
    howToUse: string;
    run(client: CustomClient, message: Message, args: string[]): Promise<any>;
}

class CustomClient extends Client {

    commands: iOfNormal[];

    constructor(options?: ClientOptions) {
        super(options);
        this.commands = [];
    }

    async loadEvents() {
        const categories = readdirSync('src/events');

        for (const category of categories) {
            const events = readdirSync(`src/events/${category}`);

            for (const event of events) {
                if (!event.endsWith('.ts') && !event.endsWith('.js')) continue;
                const eventPath = pathToFileURL(join(process.cwd(), `src/events/${category}/${event}`)).href;
                const eventModule = await import(eventPath);
                const EventClass = eventModule.default || eventModule;
                const evt = new EventClass(this);

                this.on(evt.name, evt.run);
            }
        }
    }

    async loadNormalCommands() {
        const categories = readdirSync('src/commands');

        for (const category of categories) {
            const commands = readdirSync(`src/commands/${category}`);

            for (const command of commands) {
                if (!command.endsWith('.ts') && !command.endsWith('.js')) continue;
                const commandPath = pathToFileURL(join(process.cwd(), `src/commands/${category}/${command}`)).href;
                const commandModule = await import(commandPath);
                const CommandClass = commandModule.default || commandModule;
                const cmd = new CommandClass(this);

                this.commands.push(cmd);
            }
        }
    }

    async init() {
        await this.loadEvents();
        await this.loadNormalCommands();
    }
}

export { iOfNormal, CustomClient };