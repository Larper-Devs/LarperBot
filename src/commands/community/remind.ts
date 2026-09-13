import { ChatInputCommandInteraction, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { ReminderModel } from '../../models/Reminder.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase, parseDuration } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'remind', description: 'Cria um lembrete persistente.', category: 'Convivência', data: new SlashCommandBuilder().setName('remind').setDescription('Cria um lembrete persistente.').addStringOption((option) => option.setName('duracao').setDescription('Ex.: 10m, 2h, 1d.').setRequired(true)).addStringOption((option) => option.setName('mensagem').setDescription('Texto do lembrete.').setMaxLength(1000).setRequired(true)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !interaction.channel || !await requireDatabase(client, interaction)) return
        const duration = parseDuration(interaction.options.getString('duracao', true))
        if (!duration) {
            await interaction.reply({ content: 'Duração inválida. Use `10m`, `2h`, `1d` ou `1w`.', flags: MessageFlags.Ephemeral })
            return
        }
        await ReminderModel.create({ guildId: interaction.guild.id, userId: interaction.user.id, channelId: interaction.channel.id, message: interaction.options.getString('mensagem', true), remindAt: new Date(Date.now() + duration), sent: false })
        await interaction.reply(`Lembrete criado para <t:${Math.floor((Date.now() + duration) / 1000)}:R>.`)
    }
}
