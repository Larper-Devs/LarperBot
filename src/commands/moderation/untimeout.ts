import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, PermissionFlagsBits, SlashCommandBuilder } from 'discord.js'
import { ModerationCaseModel } from '../../models/ModerationCase.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { canModerate, resolveMember } from '../../utils/moderation.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'untimeout',
            description: 'Remove o timeout de um membro.',
            category: 'Moderação',
            data: new SlashCommandBuilder()
                .setName('untimeout')
                .setDescription('Remove o timeout de um membro.')
                .addUserOption((option) => option.setName('usuario').setDescription('Membro que terá o timeout removido.').setRequired(true))
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
        await target.timeout(null, interaction.options.getString('motivo') ?? 'Timeout removido')
        await ModerationCaseModel.updateMany({ guildId: interaction.guild.id, targetId: user.id, action: 'timeout', active: true }, { $set: { active: false } })
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Timeout removido').setDescription(`${user} pode falar novamente.`).setColor(0x2ecc71)], flags: MessageFlags.Ephemeral })
    }
}
