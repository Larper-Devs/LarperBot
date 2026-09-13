import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { transferWallet } from '../../services/EconomyService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase, formatCurrency } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'pay',
            description: 'Transfere moedas para outro usuário.',
            category: 'Economia',
            data: new SlashCommandBuilder()
                .setName('pay')
                .setDescription('Transfere moedas para outro usuário.')
                .addUserOption((option) => option.setName('usuario').setDescription('Destinatário.').setRequired(true))
                .addIntegerOption((option) => option.setName('valor').setDescription('Quantidade de moedas.').setMinValue(1).setRequired(true)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        const user = interaction.options.getUser('usuario', true)
        const amount = interaction.options.getInteger('valor', true)
        if (user.bot) {
            await interaction.reply({ content: 'Não é possível transferir para bots.', flags: MessageFlags.Ephemeral })
            return
        }
        try {
            await transferWallet(interaction.guild.id, interaction.user.id, user.id, amount)
            await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Transferência concluída').setDescription(`Você enviou **${formatCurrency(amount)}** para ${user}.`).setColor(0x2ecc71)] })
        } catch (error) {
            await interaction.reply({ content: error instanceof Error ? error.message : 'Não foi possível concluir a transferência.', flags: MessageFlags.Ephemeral })
        }
    }
}
