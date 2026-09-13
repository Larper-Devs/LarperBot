import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { getLeaderboard, levelFromXp } from '../../services/LevelService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'leaderboard', description: 'Exibe o ranking de níveis.', aliases: ['rankings'], category: 'Níveis', data: new SlashCommandBuilder().setName('leaderboard').setDescription('Exibe o ranking de níveis.').addStringOption((option) => option.setName('periodo').setDescription('Período do ranking.').addChoices({ name: 'Total', value: 'total' }, { name: 'Semanal', value: 'weekly' }, { name: 'Mensal', value: 'monthly' }).setRequired(false)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        const period = (interaction.options.getString('periodo') ?? 'total') as 'total' | 'weekly' | 'monthly'
        const items = await getLeaderboard(interaction.guild.id, period)
        const field = period === 'total' ? 'totalXp' : period === 'weekly' ? 'weeklyXp' : 'monthlyXp'
        const description = items.length === 0 ? 'Ainda não há dados para este ranking.' : items.map((item, index) => `**${index + 1}.** <@${item.userId}> — ${item[field]} XP — nível ${levelFromXp(item.totalXp)}`).join('\n')
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle(`Ranking ${period === 'weekly' ? 'semanal' : period === 'monthly' ? 'mensal' : 'total'}`).setDescription(description).setColor(0xf1c40f)] })
    }
}
