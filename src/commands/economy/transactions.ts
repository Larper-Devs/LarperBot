import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { EconomyTransactionModel } from '../../models/EconomyTransaction.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase, formatCurrency } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'transactions', description: 'Consulta seu extrato de economia.', category: 'Economia', data: new SlashCommandBuilder().setName('transactions').setDescription('Consulta seu extrato de economia.') })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        const items = await EconomyTransactionModel.find({ guildId: interaction.guild.id, userId: interaction.user.id }).sort({ createdAt: -1 }).limit(10).lean()
        const description = items.length === 0 ? 'Nenhuma transação encontrada.' : items.map((item) => `${item.type} — ${formatCurrency(item.amount)} — ${item.reason}`).join('\n')
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Extrato').setDescription(description).setColor(0x5865f2)] })
    }
}
