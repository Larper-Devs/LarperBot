import {
    ChatInputCommandInteraction,
    ChannelType,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
    TextChannel,
} from 'discord.js'
import { findMemberRole, verifyPanel } from '../../services/PanelService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'verify',
            description: 'Publica o painel de verificação do servidor.',
            category: 'Utilitários',
            adminOnly: true,
            data: new SlashCommandBuilder()
                .setName('verify')
                .setDescription('Publica o painel de verificação do servidor.')
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

        const role = findMemberRole(interaction.guild)
        if (!role) {
            await interaction.reply({ content: 'Crie um cargo chamado `Membro` antes de publicar o painel.', flags: MessageFlags.Ephemeral })
            return
        }

        const channel = interaction.options.getChannel('canal') ?? interaction.channel
        if (!(channel instanceof TextChannel)) {
            await interaction.reply({ content: 'Escolha um canal de texto.', flags: MessageFlags.Ephemeral })
            return
        }

        await interaction.deferReply({ flags: MessageFlags.Ephemeral })
        await channel.send({ flags: MessageFlags.IsComponentsV2, components: [verifyPanel(role.id)] })
        await interaction.editReply({ content: `Painel de verificação publicado em ${channel}.` })
    }
}
