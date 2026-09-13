import { AttachmentBuilder, ChatInputCommandInteraction, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { renderProfileCard } from '../../services/CanvasService.js'
import { levelFromXp } from '../../services/LevelService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { getProfile } from '../../utils/profiles.js'
import { UserProfileModel } from '../../models/UserProfile.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'profile',
            description: 'Gera seu perfil visual com XP, nível e economia.',
            aliases: ['perfil'],
            category: 'Níveis',
            data: new SlashCommandBuilder()
                .setName('profile')
                .setDescription('Gera seu perfil visual com XP, nível e economia.')
                .addUserOption((option) => option.setName('usuario').setDescription('Usuário consultado.').setRequired(false)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        await interaction.deferReply()

        const user = interaction.options.getUser('usuario') ?? interaction.user
        const profile = await getProfile(interaction.guild.id, user.id, user.username)
        const rank = await UserProfileModel.countDocuments({ guildId: interaction.guild.id, totalXp: { $gt: profile.totalXp } }) + 1
        const avatarUrl = user.displayAvatarURL({ extension: 'png', size: 256 })
        const image = await renderProfileCard({
            username: user.username,
            tag: user.tag,
            avatarUrl,
            level: levelFromXp(profile.totalXp),
            totalXp: profile.totalXp,
            weeklyXp: profile.weeklyXp,
            monthlyXp: profile.monthlyXp,
            wallet: profile.wallet,
            rank,
        })

        await interaction.editReply({ files: [new AttachmentBuilder(image, { name: 'profile.png' })] })
    }
}
