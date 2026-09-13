import {
    ChatInputCommandInteraction,
    EmbedBuilder,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
} from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { canModerate, createModerationCase, resolveMember } from '../../utils/moderation.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'warn',
            description: 'Registra uma advertência para um membro.',
            category: 'Moderação',
            data: new SlashCommandBuilder()
                .setName('warn')
                .setDescription('Registra uma advertência para um membro.')
                .addUserOption((option) => option.setName('usuario').setDescription('Membro que receberá a advertência.').setRequired(true))
                .addStringOption((option) => option.setName('motivo').setDescription('Motivo da advertência.').setMaxLength(500).setRequired(true)),
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

        const reason = interaction.options.getString('motivo', true)
        const caseData = await createModerationCase({
            guildId: interaction.guild.id,
            action: 'warn',
            targetId: user.id,
            targetTag: user.tag,
            moderatorId: interaction.user.id,
            reason,
        })

        await interaction.reply({
            embeds: [new EmbedBuilder().setTitle('Advertência registrada').setDescription(`${user} recebeu uma advertência.\n**Caso:** #${caseData.caseNumber}\n**Motivo:** ${reason}`).setColor(0xf1c40f)],
            flags: MessageFlags.Ephemeral,
        })
        await user.send(`Você recebeu uma advertência em **${interaction.guild.name}**. Motivo: ${reason}`).catch(() => undefined)
    }
}
