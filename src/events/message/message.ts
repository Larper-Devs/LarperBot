import { Event } from '../../structures/Event'
import { CustomClient } from '../../structures/Client'
import { Message } from 'stoat.js'

export default class extends Event {
    constructor(client: CustomClient) {
        super(client, {
            name: 'messageCreate'
        })
    }

  run = async (message: Message) => {
        if (message.author?.bot) return;
        if (message.channel?.type === "DirectMessage") return;

        const prefix = process.env.DEFAULT_PREFIX

        try {
            if (!message.content.toLowerCase().startsWith((<string>prefix))) return;

            var args = message.content.slice((<string>prefix).length).split(' ');
            const cmd = (<string>args.shift()).toLowerCase();

            
            if (cmd.length === 0) return;

            const command = this.client.commands.find((c: { name: string }) => c.name == cmd) || this.client.commands.find((a: { aliases: string[] }) => a.aliases && a.aliases.includes(cmd));

            
            if (!command) return;
            if (command) command.run(this.client, message, args);
        } catch (error) {
            this.logger.error('Erro no processamento de mensagem', error);
        }
    }
}