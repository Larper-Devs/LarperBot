import {
    ChatInputCommandInteraction,
    EmbedBuilder,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
} from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase, parseDuration } from '../../utils/database.js'
import { canModerate, createModerationCase, resolveMember } from '../../utils/moderation.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'timeout',
            description: 'Coloca um membro em timeout.',
            category: 'Moderação',
            data: new SlashCommandBuilder()
                .setName('timeout')
                .setDescription('Coloca um membro em timeout.')
                .addUserOption((option) => option.setName('usuario').setDescription('Membro que receberá o timeout.').setRequired(true))
                .addStringOption((option) => option.setName('duracao').setDescription('Ex.: 10m, 2h, 1d. Máximo de 28 dias.').setRequired(true))
                .addStringOption((option) => option.setName('motivo').setDescription('Motivo da ação.').setMaxLength(500).setRequired(false)),
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
        const duration = parseDuration(interaction.options.getString('duracao', true))
        if (!duration) {
            await interaction.reply({ content: 'Duração inválida. Use formatos como `10m`, `2h`, `1d` ou `1w`.', flags: MessageFlags.Ephemeral })
            return
        }
        const target = await resolveMember(interaction.guild, user)
        const actor = await interaction.guild.members.fetch(interaction.user.id)
        if (!target) {
            await interaction.reply({ content: 'Não encontrei esse membro no servidor.', flags: MessageFlags.Ephemeral })
            return
        }
        const hierarchyError = canModerate(actor, target, interaction.guild.members.me)
        if (hierarchyError) {
            await interaction.reply({ content: hierarchyError, flags: MessageFlags.Ephemeral })
            return
        }

        const reason = interaction.options.getString('motivo') ?? 'Sem motivo informado'
        await target.timeout(duration, reason)
        const caseData = await createModerationCase({
            guildId: interaction.guild.id,
            action: 'timeout',
            targetId: user.id,
            targetTag: user.tag,
            moderatorId: interaction.user.id,
            reason,
            durationMs: duration,
            expiresAt: new Date(Date.now() + duration),
        })
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Timeout aplicado').setDescription(`${user} recebeu timeout.\n**Caso:** #${caseData.caseNumber}\n**Motivo:** ${reason}`).setColor(0xe67e22)], flags: MessageFlags.Ephemeral })
    }
}
