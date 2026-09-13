import { AttachmentBuilder, ChatInputCommandInteraction, SlashCommandBuilder } from 'discord.js'
import { renderRankingCard } from '../../services/CanvasService.js'
import { getLeaderboard, levelFromXp } from '../../services/LevelService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'ranking',
            description: 'Gera o ranking visual de XP do servidor.',
            aliases: ['rank-card'],
            category: 'Níveis',
            data: new SlashCommandBuilder()
                .setName('ranking')
                .setDescription('Gera o ranking visual de XP do servidor.')
                .addStringOption((option) => option
                    .setName('periodo')
                    .setDescription('Período do ranking.')
                    .addChoices({ name: 'Total', value: 'total' }, { name: 'Semanal', value: 'weekly' }, { name: 'Mensal', value: 'monthly' })
                    .setRequired(false)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        await interaction.deferReply()

        const period = (interaction.options.getString('periodo') ?? 'total') as 'total' | 'weekly' | 'monthly'
        const profiles = await getLeaderboard(interaction.guild.id, period, 10)
        const fallbackAvatar = client.user?.displayAvatarURL({ extension: 'png', size: 128 }) ?? interaction.user.displayAvatarURL({ extension: 'png', size: 128 })
        const entries = await Promise.all(profiles.map(async (profile, index) => {
            const user = await client.users.fetch(profile.userId).catch(() => null)
            const xp = period === 'total' ? profile.totalXp : period === 'weekly' ? profile.weeklyXp : profile.monthlyXp
            return {
                position: index + 1,
                username: (user?.username ?? profile.username) || 'Usuário desconhecido',
                userId: profile.userId,
                avatarUrl: user?.displayAvatarURL({ extension: 'png', size: 128 }) ?? fallbackAvatar,
                xp,
                level: levelFromXp(profile.totalXp),
            }
        }))

        const image = await renderRankingCard(period === 'weekly' ? 'semanal' : period === 'monthly' ? 'mensal' : 'total', entries)
        await interaction.editReply({ files: [new AttachmentBuilder(image, { name: 'ranking.png' })] })
    }
}
