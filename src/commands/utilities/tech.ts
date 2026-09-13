import {
    ChatInputCommandInteraction,
    ChannelType,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
    TextChannel,
} from 'discord.js'
import { technologyPanel } from '../../services/PanelService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'tech',
            description: 'Publica um painel de linguagens e tecnologias.',
            category: 'Utilitários',
            adminOnly: true,
            data: new SlashCommandBuilder()
                .setName('tech')
                .setDescription('Publica um painel de linguagens e tecnologias.')
                .addChannelOption((option) => option
                    .setName('canal')
                    .setDescription('Canal onde o painel será publicado; padrão é o atual.')
                    .addChannelTypes(ChannelType.GuildText)
                    .setRequired(false)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.Administrator)) {
            await interaction.reply({ content: 'Apenas administradores podem publicar este painel.', flags: MessageFlags.Ephemeral })
            return
        }

        const channel = interaction.options.getChannel('canal') ?? interaction.channel
        if (!(channel instanceof TextChannel)) {
            await interaction.reply({ content: 'Escolha um canal de texto.', flags: MessageFlags.Ephemeral })
            return
        }

        await interaction.deferReply({ flags: MessageFlags.Ephemeral })
        await channel.send({ flags: MessageFlags.IsComponentsV2, components: [technologyPanel()] })
        await interaction.editReply({ content: `Painel de tecnologias publicado em ${channel}.` })
    }
}
