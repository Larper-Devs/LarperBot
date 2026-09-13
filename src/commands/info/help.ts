import {
    ChatInputCommandInteraction,
    Message,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
} from 'discord.js'
import { commandHelpPanel, helpHomePanel } from '../../services/HelpService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'help',
            description: 'Abre a central de ajuda por categoria.',
            aliases: ['ajuda', 'h'],
            category: 'Informação',
            howToUse: 'help [comando]',
            data: new SlashCommandBuilder()
                .setName('help')
                .setDescription('Abre a central de ajuda por categoria.')
                .addStringOption((option) => option
                    .setName('comando')
                    .setDescription('Mostra detalhes de um comando específico.')
                    .setRequired(false)),
        })
    }

    async executeMessage(client: CustomClient, message: Message, args: string[]): Promise<void> {
        const query = args[0]?.trim()
        const canSeeAdminCommands = message.member?.permissions.has(PermissionFlagsBits.Administrator) ?? false
        await message.reply({
            flags: MessageFlags.IsComponentsV2,
            components: [query
                ? commandHelpPanel(client, message.author.id, canSeeAdminCommands, query)
                : helpHomePanel(client, message.author.id, canSeeAdminCommands)],
        })
    }

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        const query = interaction.options.getString('comando')?.trim()
        const canSeeAdminCommands = interaction.memberPermissions?.has(PermissionFlagsBits.Administrator) ?? false
        await interaction.reply({
            flags: MessageFlags.IsComponentsV2,
            components: [query
                ? commandHelpPanel(client, interaction.user.id, canSeeAdminCommands, query)
                : helpHomePanel(client, interaction.user.id, canSeeAdminCommands)],
        })
    }
}
