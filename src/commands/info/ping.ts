import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'ping', description: 'Exibe a latência do bot.', category: 'Informação', data: new SlashCommandBuilder().setName('ping').setDescription('Exibe a latência do bot.') })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Pong!').setDescription(`Gateway: **${interaction.client.ws.ping}ms**`).setColor(0x2ecc71)] })
    }
}
