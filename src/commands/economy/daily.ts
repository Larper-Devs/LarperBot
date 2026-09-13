import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { creditWallet } from '../../services/EconomyService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase, formatCurrency } from '../../utils/database.js'
import { getProfile } from '../../utils/profiles.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'daily', description: 'Resgata sua recompensa diária.', category: 'Economia', data: new SlashCommandBuilder().setName('daily').setDescription('Resgata sua recompensa diária.') })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        const profile = await getProfile(interaction.guild.id, interaction.user.id, interaction.user.username)
        const cooldown = 24 * 60 * 60 * 1_000
        if (profile.lastDailyAt && Date.now() - profile.lastDailyAt.getTime() < cooldown) {
            await interaction.reply({ content: `Você já recebeu sua recompensa. Tente novamente <t:${Math.floor((profile.lastDailyAt.getTime() + cooldown) / 1000)}:R>.`, flags: MessageFlags.Ephemeral })
            return
        }
        const amount = 500
        profile.lastDailyAt = new Date()
        await profile.save()
        await creditWallet(interaction.guild.id, interaction.user.id, amount, 'Recompensa diária', 'reward')
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Recompensa diária').setDescription(`Você recebeu **${formatCurrency(amount)}**.`).setColor(0x2ecc71)] })
    }
}
