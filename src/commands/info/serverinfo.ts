import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'serverinfo', description: 'Exibe informações do servidor.', category: 'Informação', data: new SlashCommandBuilder().setName('serverinfo').setDescription('Exibe informações do servidor.') })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild) return
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle(interaction.guild.name).setThumbnail(interaction.guild.iconURL() ?? '').addFields({ name: 'ID', value: interaction.guild.id, inline: true }, { name: 'Membros', value: String(interaction.guild.memberCount), inline: true }, { name: 'Canais', value: String(interaction.guild.channels.cache.size), inline: true }, { name: 'Criado', value: `<t:${Math.floor(interaction.guild.createdTimestamp / 1000)}:D>` }).setColor(0x5865f2)] })
    }
}
