import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { levelFromXp } from '../../services/LevelService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { getProfile } from '../../utils/profiles.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'level', description: 'Consulta seu nível e experiência.', aliases: ['rank'], category: 'Níveis', data: new SlashCommandBuilder().setName('level').setDescription('Consulta seu nível e experiência.').addUserOption((option) => option.setName('usuario').setDescription('Usuário consultado.').setRequired(false)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        const user = interaction.options.getUser('usuario') ?? interaction.user
        const profile = await getProfile(interaction.guild.id, user.id, user.username)
        const level = levelFromXp(profile.totalXp)
        const nextXp = (level + 1) ** 2 * 100
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle(`Nível de ${user.username}`).setDescription(`**Nível:** ${level}\n**XP total:** ${profile.totalXp}/${nextXp}\n**Semanal:** ${profile.weeklyXp}\n**Mensal:** ${profile.monthlyXp}`).setThumbnail(user.displayAvatarURL()).setColor(0x5865f2)] })
    }
}
