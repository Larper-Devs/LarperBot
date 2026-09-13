import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, PermissionFlagsBits, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { createModerationCase } from '../../utils/moderation.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'unban',
            description: 'Remove o banimento de um usuário pelo ID.',
            category: 'Moderação',
            data: new SlashCommandBuilder()
                .setName('unban')
                .setDescription('Remove o banimento de um usuário pelo ID.')
                .addStringOption((option) => option.setName('usuario_id').setDescription('ID do usuário banido.').setRequired(true))
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
        const userId = interaction.options.getString('usuario_id', true)
        const reason = interaction.options.getString('motivo') ?? 'Sem motivo informado'
        try {
            const ban = await interaction.guild.bans.fetch(userId)
            await interaction.guild.members.unban(userId, reason)
            const caseData = await createModerationCase({ guildId: interaction.guild.id, action: 'unban', targetId: userId, targetTag: ban.user.tag, moderatorId: interaction.user.id, reason })
            await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Banimento removido').setDescription(`${ban.user.tag} pode entrar novamente.\n**Caso:** #${caseData.caseNumber}`).setColor(0x2ecc71)], flags: MessageFlags.Ephemeral })
        } catch {
            await interaction.reply({ content: 'Esse usuário não está banido ou o ID é inválido.', flags: MessageFlags.Ephemeral })
        }
    }
}
