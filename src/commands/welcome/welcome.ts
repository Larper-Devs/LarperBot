import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, PermissionFlagsBits, SlashCommandBuilder, TextChannel } from 'discord.js'
import { GuildConfigModel } from '../../models/GuildConfig.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { colorPanel, sendWelcome } from '../../services/WelcomeService.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'welcome',
            description: 'Configura boas-vindas, saídas e painéis de cores.',
            category: 'Boas-vindas',
            data: new SlashCommandBuilder()
                .setName('welcome')
                .setDescription('Configura boas-vindas, saídas e painéis de cores.')
                .addSubcommand((subcommand) => subcommand
                    .setName('config')
                    .setDescription('Configura a mensagem de entrada.')
                    .addBooleanOption((option) => option.setName('ativado').setDescription('Ativa a mensagem.').setRequired(true))
                    .addChannelOption((option) => option.setName('canal').setDescription('Canal de entrada.').setRequired(false))
                    .addStringOption((option) => option.setName('mensagem').setDescription('Mensagem com {mention}, {server}, {memberCount}.').setMaxLength(1000).setRequired(false)))
                .addSubcommand((subcommand) => subcommand.setName('disable').setDescription('Desativa boas-vindas.'))
                .addSubcommand((subcommand) => subcommand.setName('test').setDescription('Envia uma prévia da mensagem.'))
                .addSubcommand((subcommand) => subcommand
                    .setName('color-role')
                    .setDescription('Adiciona ou remove uma role do painel de cores.')
                    .addStringOption((option) => option.setName('acao').setDescription('Operação.').addChoices({ name: 'Adicionar', value: 'add' }, { name: 'Remover', value: 'remove' }).setRequired(true))
                    .addRoleOption((option) => option.setName('cargo').setDescription('Cargo de cor.').setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('panel')
                    .setDescription('Publica o painel de escolha de cores.')
                    .addChannelOption((option) => option.setName('canal').setDescription('Canal do painel.').setRequired(true))),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.ManageGuild)) {
            await interaction.reply({ content: 'Você precisa da permissão `Gerenciar Servidor`.', flags: MessageFlags.Ephemeral })
            return
        }
        if (!await requireDatabase(client, interaction)) return
        const config = await GuildConfigModel.findOneAndUpdate({ guildId: interaction.guild.id }, { $setOnInsert: { guildId: interaction.guild.id } }, { returnDocument: 'after', upsert: true, setDefaultsOnInsert: true })
        if (!config) throw new Error('Não foi possível carregar a configuração.')
        const subcommand = interaction.options.getSubcommand()

        if (subcommand === 'config') {
            config.welcome.enabled = interaction.options.getBoolean('ativado', true)
            const channel = interaction.options.getChannel('canal')
            const message = interaction.options.getString('mensagem')
            if (channel) config.welcome.channelId = channel.id
            if (message) config.welcome.message = message
            await config.save()
            await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Boas-vindas atualizadas').setDescription(`Status: **${config.welcome.enabled ? 'ativado' : 'desativado'}**\nCanal: ${config.welcome.channelId ? `<#${config.welcome.channelId}>` : 'não definido'}`).setColor(0x5865f2)], flags: MessageFlags.Ephemeral })
            return
        }
        if (subcommand === 'disable') {
            config.welcome.enabled = false
            await config.save()
            await interaction.reply({ content: 'Boas-vindas desativadas.', flags: MessageFlags.Ephemeral })
            return
        }
        if (subcommand === 'color-role') {
            const role = interaction.options.getRole('cargo', true)
            const action = interaction.options.getString('acao', true)
            if (action === 'add' && !config.welcome.colorRoleIds.includes(role.id)) config.welcome.colorRoleIds.push(role.id)
            if (action === 'remove') config.welcome.colorRoleIds = config.welcome.colorRoleIds.filter((id: string) => id !== role.id)
            await config.save()
            await interaction.reply({ content: `Cargos disponíveis no painel: ${config.welcome.colorRoleIds.length}.`, flags: MessageFlags.Ephemeral })
            return
        }
        if (subcommand === 'panel') {
            const channel = interaction.options.getChannel('canal', true)
            if (!(channel instanceof TextChannel)) {
                await interaction.reply({ content: 'Escolha um canal de texto.', flags: MessageFlags.Ephemeral })
                return
            }
            if (config.welcome.colorRoleIds.length === 0) {
                await interaction.reply({ content: 'Adicione pelo menos um cargo com `/welcome color-role` antes de publicar o painel.', flags: MessageFlags.Ephemeral })
                return
            }
            const message = await channel.send({ embeds: [new EmbedBuilder().setTitle('Escolha sua cor').setDescription('Selecione uma cor no menu abaixo.').setColor(0x5865f2)], components: [colorPanel(config.welcome)] })
            config.welcome.panelChannelId = channel.id
            config.welcome.panelMessageId = message.id
            await config.save()
            await interaction.reply({ content: `Painel publicado em ${channel}.`, flags: MessageFlags.Ephemeral })
            return
        }

        const member = await interaction.guild.members.fetch(interaction.user.id)
        await sendWelcome(client, member)
        await interaction.reply({ content: 'Prévia enviada.', flags: MessageFlags.Ephemeral })
    }
}
