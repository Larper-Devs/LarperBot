import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, PermissionFlagsBits, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { canModerate, createModerationCase, resolveMember } from '../../utils/moderation.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'kick',
            description: 'Expulsa um membro do servidor.',
            category: 'Moderação',
            data: new SlashCommandBuilder()
                .setName('kick')
                .setDescription('Expulsa um membro do servidor.')
                .addUserOption((option) => option.setName('usuario').setDescription('Membro que será expulso.').setRequired(true))
                .addStringOption((option) => option.setName('motivo').setDescription('Motivo da ação.').setMaxLength(500).setRequired(false)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.KickMembers)) {
            await interaction.reply({ content: 'Você precisa da permissão `Expulsar Membros`.', flags: MessageFlags.Ephemeral })
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
        await target.kick(reason)
        const caseData = await createModerationCase({ guildId: interaction.guild.id, action: 'kick', targetId: user.id, targetTag: user.tag, moderatorId: interaction.user.id, reason })
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Membro expulso').setDescription(`${user.tag} foi expulso.\n**Caso:** #${caseData.caseNumber}\n**Motivo:** ${reason}`).setColor(0xe67e22)], flags: MessageFlags.Ephemeral })
    }
}
