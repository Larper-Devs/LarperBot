import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, PermissionFlagsBits, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { canModerate, createModerationCase, resolveMember } from '../../utils/moderation.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'ban',
            description: 'Bane um membro do servidor.',
            category: 'Moderação',
            data: new SlashCommandBuilder()
                .setName('ban')
                .setDescription('Bane um membro do servidor.')
                .addUserOption((option) => option.setName('usuario').setDescription('Membro que será banido.').setRequired(true))
                .addStringOption((option) => option.setName('motivo').setDescription('Motivo da ação.').setMaxLength(500).setRequired(false)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.BanMembers)) {
            await interaction.reply({ content: 'Você precisa da permissão `Banir Membros`.', flags: MessageFlags.Ephemeral })
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
        const reason = interaction.options.getString('motivo') ?? 'Sem motivo informado'
        await target.ban({ reason, deleteMessageSeconds: 0 })
        const caseData = await createModerationCase({ guildId: interaction.guild.id, action: 'ban', targetId: user.id, targetTag: user.tag, moderatorId: interaction.user.id, reason })
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Membro banido').setDescription(`${user.tag} foi banido.\n**Caso:** #${caseData.caseNumber}\n**Motivo:** ${reason}`).setColor(0xed4245)], flags: MessageFlags.Ephemeral })
    }
}
