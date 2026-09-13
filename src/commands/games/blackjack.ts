import { ChatInputCommandInteraction, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { startBlackjack } from '../../services/GameService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'blackjack', description: 'Joga blackjack com moedas da economia.', category: 'Jogos', data: new SlashCommandBuilder().setName('blackjack').setDescription('Joga blackjack com moedas da economia.').addIntegerOption((option) => option.setName('aposta').setDescription('Valor da aposta.').setMinValue(1).setRequired(true)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        try {
            await startBlackjack(interaction, interaction.options.getInteger('aposta', true))
        } catch (error) {
            await interaction.reply({ content: error instanceof Error ? error.message : 'Não foi possível iniciar o jogo.', flags: MessageFlags.Ephemeral })
        }
    }
}
