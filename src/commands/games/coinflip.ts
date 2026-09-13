import { ChatInputCommandInteraction, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { coinflip } from '../../services/GameService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'coinflip', description: 'Aposta em cara ou coroa.', category: 'Jogos', data: new SlashCommandBuilder().setName('coinflip').setDescription('Aposta em cara ou coroa.').addStringOption((option) => option.setName('escolha').setDescription('Sua escolha.').addChoices({ name: 'Cara', value: 'cara' }, { name: 'Coroa', value: 'coroa' }).setRequired(true)).addIntegerOption((option) => option.setName('aposta').setDescription('Valor da aposta.').setMinValue(1).setRequired(true)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        try {
            await coinflip(interaction, interaction.options.getInteger('aposta', true), interaction.options.getString('escolha', true) as 'cara' | 'coroa')
        } catch (error) {
            await interaction.reply({ content: error instanceof Error ? error.message : 'Não foi possível realizar a aposta.', flags: MessageFlags.Ephemeral })
        }
    }
}
