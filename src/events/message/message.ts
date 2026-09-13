import { Event } from '../../structures/Event.js'
import { CustomClient } from '../../structures/Client.js'
import { Message, PermissionFlagsBits } from 'discord.js'
import { handleMessage as handleAutomod } from '../../services/AutomodService.js'
import { processMessage as processLevel } from '../../services/LevelService.js'

export default class extends Event {
    constructor(client: CustomClient) {
        super(client, {
            name: 'messageCreate'
        })
    }

    run = async (message: Message) => {
        if (message.author?.bot) return;
        if (!message.inGuild()) return;

        if (this.client.automodEnabled) {
            const moderated = await handleAutomod(this.client, message)
            if (moderated) return
        }

        if (this.client.levelsEnabled) {
            await processLevel(this.client, message)
        }

        if (!this.client.prefixCommandsEnabled) return;

        const prefix = this.client.prefix;
        if (!message.content.startsWith(prefix)) return;

        try {
            const tokens = message.content.slice(prefix.length).trim().split(/\s+/);
            const cmd = tokens.shift()?.toLowerCase();
            const args = tokens.filter(Boolean);

            if (!cmd) return;

            const command = this.client.findCommand(cmd);

            if (!command) return;

            if (command.adminOnly && !message.member?.permissions.has(PermissionFlagsBits.Administrator)) {
                await message.reply('Apenas administradores podem usar este comando.')
                return
            }

            await command.executeMessage(this.client, message, args);
        } catch (error) {
            this.logger.error('Erro no processamento de mensagem', error);
            await message.reply('Ocorreu um erro ao executar esse comando.').catch(() => undefined);
        }
    }
}
