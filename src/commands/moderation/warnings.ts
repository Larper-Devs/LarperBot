import {
    ChatInputCommandInteraction,
    EmbedBuilder,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
} from 'discord.js'
import { ModerationCaseModel } from '../../models/ModerationCase.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'warnings',
            description: 'Consulta advertências de um membro.',
            aliases: ['warns'],
            category: 'Moderação',
            data: new SlashCommandBuilder()
                .setName('warnings')
                .setDescription('Consulta advertências de um membro.')
                .addUserOption((option) => option.setName('usuario').setDescription('Membro consultado.').setRequired(true)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.ModerateMembers)) {
            await interaction.reply({ content: 'Você precisa da permissão `Moderar Membros`.', flags: MessageFlags.Ephemeral })
            return
        }
        if (!await requireDatabase(client, interaction)) return

        const user = interaction.options.getUser('usuario', true)
        const cases = await ModerationCaseModel.find({ guildId: interaction.guild.id, targetId: user.id, action: 'warn' }).sort({ createdAt: -1 }).limit(10).lean()
        const description = cases.length === 0
            ? `${user} não possui advertências registradas.`
            : cases.map((item) => `**#${item.caseNumber}** — <@${item.moderatorId}> — ${item.reason}`).join('\n')
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle(`Advertências de ${user.tag}`).setDescription(description).setColor(0xf1c40f)], flags: MessageFlags.Ephemeral })
    }
}
