import { ChatInputCommandInteraction, MessageFlags, PermissionFlagsBits, SlashCommandBuilder, TextChannel } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'unlock', description: 'Desbloqueia mensagens em um canal.', category: 'Moderação', data: new SlashCommandBuilder().setName('unlock').setDescription('Desbloqueia mensagens em um canal.').addChannelOption((option) => option.setName('canal').setDescription('Canal desbloqueado; padrão é o atual.').setRequired(false)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.ManageChannels)) {
            await interaction.reply({ content: 'Você precisa da permissão `Gerenciar Canais`.', flags: MessageFlags.Ephemeral })
            return
        }
        const channel = interaction.options.getChannel('canal') ?? interaction.channel
        if (!(channel instanceof TextChannel)) {
            await interaction.reply({ content: 'Escolha um canal de texto.', flags: MessageFlags.Ephemeral })
            return
        }
        await channel.permissionOverwrites.edit(interaction.guild.roles.everyone, { SendMessages: null }, { reason: `Desbloqueado por ${interaction.user.tag}` })
        await interaction.reply(`🔓 ${channel} foi desbloqueado.`)
    }
}
