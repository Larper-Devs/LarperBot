import {
    ChatInputCommandInteraction,
    ChannelType,
    EmbedBuilder,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
    TextChannel,
} from 'discord.js'
import type { RuleItem } from '../../models/RulesConfig.js'
import { getRulesConfig, rulesEmbed, syncRulesMessage } from '../../services/RulesService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'regras',
            description: 'Cria e gerencia o embed de regras do servidor.',
            category: 'Utilitários',
            adminOnly: true,
            data: new SlashCommandBuilder()
                .setName('regras')
                .setDescription('Cria e gerencia o embed de regras do servidor.')
                .addSubcommand((subcommand) => subcommand
                    .setName('enviar')
                    .setDescription('Envia as regras para um canal específico.')
                    .addChannelOption((option) => option
                        .setName('canal')
                        .setDescription('Canal onde as regras serão publicadas.')
                        .addChannelTypes(ChannelType.GuildText)
                        .setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('adicionar')
                    .setDescription('Adiciona uma regra ao prefab atual.')
                    .addStringOption((option) => option.setName('titulo').setDescription('Título da regra.').setMaxLength(100).setRequired(true))
                    .addStringOption((option) => option.setName('texto').setDescription('Descrição da regra.').setMaxLength(1_000).setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('editar')
                    .setDescription('Edita uma regra pelo ID.')
                    .addIntegerOption((option) => option.setName('id').setDescription('ID da regra.').setMinValue(1).setRequired(true))
                    .addStringOption((option) => option.setName('titulo').setDescription('Novo título.').setMaxLength(100).setRequired(false))
                    .addStringOption((option) => option.setName('texto').setDescription('Novo texto.').setMaxLength(1_000).setRequired(false)))
                .addSubcommand((subcommand) => subcommand
                    .setName('remover')
                    .setDescription('Remove uma regra pelo ID ou todas com all.')
                    .addStringOption((option) => option.setName('id').setDescription('ID da regra ou all.').setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('config')
                    .setDescription('Configura título, descrição e cor do embed.')
                    .addStringOption((option) => option.setName('titulo').setDescription('Título do embed.').setMaxLength(256).setRequired(false))
                    .addStringOption((option) => option.setName('descricao').setDescription('Descrição do embed.').setMaxLength(4_000).setRequired(false))
                    .addStringOption((option) => option.setName('cor').setDescription('Cor hexadecimal, como #8B5CF6.').setRequired(false)))
                .addSubcommand((subcommand) => subcommand
                    .setName('listar')
                    .setDescription('Lista as regras atuais e seus IDs.')),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.Administrator)) {
            await interaction.reply({ content: 'Apenas administradores podem gerenciar as regras.', flags: MessageFlags.Ephemeral })
            return
        }
        await interaction.deferReply({ flags: MessageFlags.Ephemeral })
        if (!await requireDatabase(client, interaction)) return

        const config = await getRulesConfig(interaction.guild.id)
        const subcommand = interaction.options.getSubcommand()

        if (subcommand === 'enviar') {
            const channel = interaction.options.getChannel('canal', true)
            if (!(channel instanceof TextChannel)) {
                await interaction.editReply({ content: 'Escolha um canal de texto.' })
                return
            }

            const message = await channel.send({ embeds: [rulesEmbed(config)] })
            config.channelId = channel.id
            config.messageId = message.id
            await config.save()
            await interaction.editReply({ content: `Embed de regras enviado para ${channel}.` })
            return
        }

        if (subcommand === 'adicionar') {
            if (config.items.length >= 25) {
                await interaction.editReply({ content: 'O Discord permite no máximo 25 regras neste embed.' })
                return
            }

            const title = interaction.options.getString('titulo', true).trim()
            const description = interaction.options.getString('texto', true).trim()
            let id = 1
            while (config.items.some((rule: RuleItem) => rule.id === id)) id += 1
            config.items.push({ id, title, description })
            await config.save()
            await syncRulesMessage(client, config)
            await interaction.editReply({ content: `Regra **${id}** adicionada.` })
            return
        }

        if (subcommand === 'editar') {
            const id = interaction.options.getInteger('id', true)
            const rule = config.items.find((item: RuleItem) => item.id === id)
            if (!rule) {
                await interaction.editReply({ content: `A regra **${id}** não existe.` })
                return
            }

            const title = interaction.options.getString('titulo')?.trim()
            const description = interaction.options.getString('texto')?.trim()
            if (!title && !description) {
                await interaction.editReply({ content: 'Informe `titulo`, `texto` ou ambos para editar a regra.' })
                return
            }
            if (title) rule.title = title
            if (description) rule.description = description
            await config.save()
            await syncRulesMessage(client, config)
            await interaction.editReply({ content: `Regra **${id}** editada.` })
            return
        }

        if (subcommand === 'remover') {
            const idInput = interaction.options.getString('id', true).trim().toLowerCase()
            if (idInput === 'all') {
                config.items = []
                await config.save()
                await syncRulesMessage(client, config)
                await interaction.editReply({ content: 'Todas as regras foram removidas.' })
                return
            }

            const id = Number.parseInt(idInput, 10)
            if (!Number.isInteger(id) || id < 1) {
                await interaction.editReply({ content: 'Informe um ID numérico válido ou `all`.' })
                return
            }
            const previousLength = config.items.length
            config.items = config.items.filter((rule: RuleItem) => rule.id !== id)
            if (config.items.length === previousLength) {
                await interaction.editReply({ content: `A regra **${id}** não existe.` })
                return
            }
            await config.save()
            await syncRulesMessage(client, config)
            await interaction.editReply({ content: `Regra **${id}** removida.` })
            return
        }

        if (subcommand === 'config') {
            const title = interaction.options.getString('titulo')?.trim()
            const description = interaction.options.getString('descricao')?.trim()
            const rawColor = interaction.options.getString('cor')?.trim()
            if (!title && !description && !rawColor) {
                await interaction.editReply({ content: 'Informe pelo menos uma configuração para alterar.' })
                return
            }
            if (title) config.title = title
            if (description) config.description = description
            if (rawColor) {
                const color = rawColor.startsWith('#') ? rawColor : `#${rawColor}`
                if (!/^#[0-9a-f]{6}$/i.test(color)) {
                    await interaction.editReply({ content: 'A cor deve estar no formato hexadecimal, como `#8B5CF6`.' })
                    return
                }
                config.color = color.toUpperCase()
            }
            await config.save()
            await syncRulesMessage(client, config)
            await interaction.editReply({ content: 'Configuração do embed de regras atualizada.' })
            return
        }

        const description = config.items.length === 0
            ? 'Nenhuma regra cadastrada.'
            : config.items.map((rule: RuleItem) => `**${rule.id}. ${rule.title}**\n${rule.description}`).join('\n\n')
        await interaction.editReply({
            embeds: [new EmbedBuilder().setTitle(config.title).setDescription(description).setColor(config.color as `#${string}`)],
        })
    }
}
