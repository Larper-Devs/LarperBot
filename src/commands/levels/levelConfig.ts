import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, PermissionFlagsBits, SlashCommandBuilder } from 'discord.js'
import { GuildConfigModel } from '../../models/GuildConfig.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'level-config',
            description: 'Configura o sistema de níveis.',
            category: 'Níveis',
            data: new SlashCommandBuilder()
                .setName('level-config')
                .setDescription('Configura o sistema de níveis.')
                .addBooleanOption((option) => option.setName('ativado').setDescription('Ativa o ganho de XP.').setRequired(true))
                .addChannelOption((option) => option.setName('canal_anuncio').setDescription('Canal para anúncios de level up.').setRequired(false))
                .addIntegerOption((option) => option.setName('cooldown').setDescription('Cooldown de XP em segundos.').setMinValue(5).setMaxValue(3600).setRequired(false)),
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
        config.level.enabled = interaction.options.getBoolean('ativado', true)
        const channel = interaction.options.getChannel('canal_anuncio')
        const cooldown = interaction.options.getInteger('cooldown')
        if (channel) config.level.announceChannelId = channel.id
        if (cooldown !== null) config.level.xpCooldownSeconds = cooldown
        await config.save()
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Sistema de níveis atualizado').setDescription(`Status: **${config.level.enabled ? 'ativado' : 'desativado'}**\nCooldown: ${config.level.xpCooldownSeconds}s`).setColor(0x5865f2)], flags: MessageFlags.Ephemeral })
    }
}
