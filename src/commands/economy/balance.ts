import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase, formatCurrency } from '../../utils/database.js'
import { getProfile } from '../../utils/profiles.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'balance', description: 'Consulta o saldo de um usuário.', aliases: ['bal'], category: 'Economia', data: new SlashCommandBuilder().setName('balance').setDescription('Consulta o saldo de um usuário.').addUserOption((option) => option.setName('usuario').setDescription('Usuário consultado.').setRequired(false)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        const user = interaction.options.getUser('usuario') ?? interaction.user
        const profile = await getProfile(interaction.guild.id, user.id, user.username)
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle(`Saldo de ${user.username}`).addFields({ name: 'Carteira', value: formatCurrency(profile.wallet), inline: true }, { name: 'Banco', value: formatCurrency(profile.bank), inline: true }, { name: 'Total', value: formatCurrency(profile.wallet + profile.bank), inline: true }).setColor(0x2ecc71)] })
    }
}
