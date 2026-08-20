import { readdirSync } from "fs";
import { join } from "path";

import { Client, Message, ClientOptions } from 'stoat.js';
import { createRequire } from "module";
const require = createRequire(import.meta.url);

interface iOfNormal {
    name: string;
    description: string;
    category: string;
    aliases: string[];
    howToUse: string;
    run(client: CustomClient, message: Message, args: string[]): Promise<void>
}

class CustomClient extends Client {

    commands: iOfNormal[];

    constructor(options?: ClientOptions) {
        super(options)

        this.commands = [];
        this.loadEvents();
        this.loadNormalCommands();
    }

    loadEvents() {
        const categories = readdirSync('src/events')

        for (const category of categories) {
            const events = readdirSync(`src/events/${category}`)

            for (const event of events) {
                const eventModule = require(join(process.cwd(), `src/events/${category}/${event}`));
                const EventClass = eventModule.default || eventModule;
                const evt = new EventClass(this);

                this.on(evt.name, evt.run)
            }
        }
    }

    loadNormalCommands() {
        const categories = readdirSync('src/commands')

        for (const category of categories) {
            const commands = readdirSync(`src/commands/${category}`)

            for (const command of commands) {
                const commandModule = require(join(process.cwd(), `src/commands/${category}/${command}`));
                const CommandClass = commandModule.default || commandModule;
                const cmd = new CommandClass(this);

                this.commands.push(cmd) || (<iOfNormal[]>this.commands).filter((a: iOfNormal) => {
                    var position = a.aliases.indexOf(cmd.aliases)
                    return a.aliases[position];
                }).push(cmd.aliases)
            }
        }
    }
}

export { iOfNormal, CustomClient }