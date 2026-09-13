import {
    ChatInputCommandInteraction,
    EmbedBuilder,
    Message,
    MessageFlags,
    SlashCommandBuilder,
} from 'discord.js'
import { CalculatorError, calculateExpression, formatCalculationResult } from '../../services/CalculatorService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'calc',
            description: 'Calcula uma expressão matemática.',
            category: 'Utilitários',
            data: new SlashCommandBuilder()
                .setName('calc')
                .setDescription('Calcula uma expressão matemática.')
                .addStringOption((option) => option
                    .setName('expressao')
                    .setDescription('Ex.: (10 + 5) * 2 / 3')
                    .setMaxLength(200)
                    .setRequired(true)),
        })
    }

    async executeMessage(_client: CustomClient, message: Message, args: string[]): Promise<void> {
        const expression = args.join(' ').trim()
        await message.reply({ embeds: [this.createEmbed(expression)] })
    }

    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        const expression = interaction.options.getString('expressao', true)

        try {
            await interaction.reply({ embeds: [this.createEmbed(expression)] })
        } catch (error) {
            const message = error instanceof CalculatorError ? error.message : 'Não foi possível calcular essa expressão.'
            await interaction.reply({ content: message, flags: MessageFlags.Ephemeral }).catch(() => undefined)
        }
    }

    private createEmbed(expression: string): EmbedBuilder {
        try {
            const result = calculateExpression(expression)
            return new EmbedBuilder()
                .setTitle('Calculadora')
                .addFields(
                    { name: 'Expressão', value: `\`${expression || 'vazia'}\`` },
                    { name: 'Resultado', value: `**${formatCalculationResult(result)}**` },
                )
                .setColor(0x8b5cf6)
        } catch (error) {
            const message = error instanceof CalculatorError ? error.message : 'Não foi possível calcular essa expressão.'
            return new EmbedBuilder()
                .setTitle('Calculadora')
                .setDescription(message)
                .setColor(0xed4245)
        }
    }
}
