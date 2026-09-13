import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, PermissionFlagsBits, SlashCommandBuilder } from 'discord.js'
import { GuildConfigModel } from '../../models/GuildConfig.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'automod',
            description: 'Configura a moderação automática do servidor.',
            category: 'Moderação',
            data: new SlashCommandBuilder()
                .setName('automod')
                .setDescription('Configura a moderação automática do servidor.')
                .addSubcommand((subcommand) => subcommand
                    .setName('config')
                    .setDescription('Ativa ou desativa o automod e ajusta limites.')
                    .addBooleanOption((option) => option.setName('ativado').setDescription('Ativa o automod.').setRequired(true))
                    .addIntegerOption((option) => option.setName('spam_limite').setDescription('Mensagens na janela de spam.').setMinValue(3).setMaxValue(20).setRequired(false))
                    .addIntegerOption((option) => option.setName('mencoes_limite').setDescription('Menções máximas por mensagem.').setMinValue(2).setMaxValue(20).setRequired(false)))
                .addSubcommand((subcommand) => subcommand
                    .setName('palavra')
                    .setDescription('Adiciona ou remove uma palavra bloqueada.')
                    .addStringOption((option) => option.setName('acao').setDescription('Operação.').addChoices({ name: 'Adicionar', value: 'add' }, { name: 'Remover', value: 'remove' }).setRequired(true))
                    .addStringOption((option) => option.setName('valor').setDescription('Palavra ou expressão.').setMaxLength(100).setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('dominio')
                    .setDescription('Adiciona ou remove um domínio bloqueado.')
                    .addStringOption((option) => option.setName('acao').setDescription('Operação.').addChoices({ name: 'Adicionar', value: 'add' }, { name: 'Remover', value: 'remove' }).setRequired(true))
                    .addStringOption((option) => option.setName('valor').setDescription('Domínio, como exemplo.com.').setMaxLength(100).setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('whitelist')
                    .setDescription('Adiciona ou remove canal/cargo da whitelist.')
                    .addStringOption((option) => option.setName('tipo').setDescription('Tipo da whitelist.').addChoices({ name: 'Canal', value: 'channel' }, { name: 'Cargo', value: 'role' }).setRequired(true))
                    .addStringOption((option) => option.setName('acao').setDescription('Operação.').addChoices({ name: 'Adicionar', value: 'add' }, { name: 'Remover', value: 'remove' }).setRequired(true))
                    .addStringOption((option) => option.setName('id').setDescription('ID do canal ou cargo.').setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('log')
                    .setDescription('Define o canal de logs do automod.')
                    .addChannelOption((option) => option.setName('canal').setDescription('Canal de logs.').setRequired(true))),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.memberPermissions?.has(PermissionFlagsBits.ManageGuild)) {
            await interaction.reply({ content: 'Você precisa da permissão `Gerenciar Servidor`.', flags: MessageFlags.Ephemeral })
            return
        }
        if (!await requireDatabase(client, interaction)) return

        const subcommand = interaction.options.getSubcommand()
        const config = await GuildConfigModel.findOneAndUpdate({ guildId: interaction.guild.id }, { $setOnInsert: { guildId: interaction.guild.id } }, { returnDocument: 'after', upsert: true, setDefaultsOnInsert: true })
        if (!config) throw new Error('Não foi possível carregar a configuração.')

        if (subcommand === 'config') {
            config.automod.enabled = interaction.options.getBoolean('ativado', true)
            const spamLimit = interaction.options.getInteger('spam_limite')
            const mentionLimit = interaction.options.getInteger('mencoes_limite')
            if (spamLimit !== null) config.automod.spamLimit = spamLimit
            if (mentionLimit !== null) config.automod.mentionLimit = mentionLimit
            await config.save()
            await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Automod atualizado').setDescription(`Status: **${config.automod.enabled ? 'ativado' : 'desativado'}**\nLimite de spam: ${config.automod.spamLimit}\nLimite de menções: ${config.automod.mentionLimit}`).setColor(0x5865f2)], flags: MessageFlags.Ephemeral })
            return
        }

        if (subcommand === 'log') {
            const channel = interaction.options.getChannel('canal', true)
            config.logging.automodChannelId = channel.id
            await config.save()
            await interaction.reply({ content: `Logs do automod configurados em ${channel}.`, flags: MessageFlags.Ephemeral })
            return
        }

        if (subcommand === 'whitelist') {
            const type = interaction.options.getString('tipo', true)
            const action = interaction.options.getString('acao', true)
            const id = interaction.options.getString('id', true)
            const list = type === 'channel' ? config.automod.exemptChannelIds : config.automod.exemptRoleIds
            if (action === 'add' && !list.includes(id)) list.push(id)
            if (action === 'remove') {
                const index = list.indexOf(id)
                if (index >= 0) list.splice(index, 1)
            }
            await config.save()
            await interaction.reply({ content: `Whitelist de ${type === 'channel' ? 'canais' : 'cargos'}: ${list.length}.`, flags: MessageFlags.Ephemeral })
            return
        }

        const action = interaction.options.getString('acao', true)
        const value = interaction.options.getString('valor', true).toLowerCase().trim()
        const list = subcommand === 'palavra' ? config.automod.blockedWords : config.automod.blockedDomains
        if (action === 'add' && !list.includes(value)) list.push(value)
        if (action === 'remove') {
            const index = list.indexOf(value)
            if (index >= 0) list.splice(index, 1)
        }
        await config.save()
        await interaction.reply({ content: `${subcommand === 'palavra' ? 'Palavras' : 'Domínios'} bloqueados: ${list.length}.`, flags: MessageFlags.Ephemeral })
    }
}
