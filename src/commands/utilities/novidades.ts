import {
    ChatInputCommandInteraction,
    ChannelType,
    EmbedBuilder,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
    TextChannel,
} from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'novidades',
            description: 'Publica um embed de novidades no servidor.',
            category: 'Utilitários',
            adminOnly: true,
            data: new SlashCommandBuilder()
                .setName('novidades')
                .setDescription('Publica um embed de novidades no servidor.')
                .addChannelOption((option) => option
                    .setName('canal')
                    .setDescription('Canal onde a novidade será publicada; padrão é o atual.')
                    .addChannelTypes(ChannelType.GuildText)
                    .setRequired(false))
                .addStringOption((option) => option
                    .setName('titulo')
                    .setDescription('Título da novidade.')
                    .setMaxLength(256)
                    .setRequired(false))
                .addStringOption((option) => option
                    .setName('descricao')
                    .setDescription('Texto da novidade.')
                    .setMaxLength(4_000)
                    .setRequired(false)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.Administrator)) {
            await interaction.reply({ content: 'Apenas administradores podem publicar novidades.', flags: MessageFlags.Ephemeral })
            return
        }

        const channel = interaction.options.getChannel('canal') ?? interaction.channel
        if (!(channel instanceof TextChannel)) {
            await interaction.reply({ content: 'Escolha um canal de texto.', flags: MessageFlags.Ephemeral })
            return
        }

        const title = interaction.options.getString('titulo')?.trim() || 'Novidades'
        const description = interaction.options.getString('descricao')?.trim() || 'Fique de olho neste canal para acompanhar as novidades do servidor.'
        await interaction.deferReply({ flags: MessageFlags.Ephemeral })
        await channel.send({
            embeds: [new EmbedBuilder()
                .setTitle(title)
                .setDescription(description)
                .setColor(0x8b5cf6)
                .setFooter({ text: 'LarperBot • Atualizações do servidor' })],
        })
        await interaction.editReply({ content: `Embed de novidades publicado em ${channel}.` })
    }
}
