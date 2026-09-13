import {
    AutocompleteInteraction,
    ChatInputCommandInteraction,
    EmbedBuilder,
    MessageFlags,
    SlashCommandBuilder,
} from 'discord.js'
import { calculateDevex, DEVEX_USD_PER_ROBUX, formatMoney, searchCurrencies } from '../../services/DevexService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'devex',
            description: 'Calcula o valor de Robux no Developer Exchange.',
            category: 'Utilitários',
            data: new SlashCommandBuilder()
                .setName('devex')
                .setDescription('Calcula o valor de Robux no Developer Exchange.')
                .addIntegerOption((option) => option
                    .setName('robux')
                    .setDescription('Quantidade de Robux ganhos.')
                    .setMinValue(1)
                    .setRequired(true))
                .addStringOption((option) => option
                    .setName('moeda')
                    .setDescription('Código ISO 4217 da moeda de destino, como BRL ou USD.')
                    .setAutocomplete(true)
                    .setRequired(true)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeAutocomplete(_client: CustomClient, interaction: AutocompleteInteraction): Promise<void> {
        await interaction.respond(await searchCurrencies(interaction.options.getFocused()))
    }

    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        const robux = interaction.options.getInteger('robux', true)
        const currency = interaction.options.getString('moeda', true)

        try {
            const result = await calculateDevex(robux, currency)
            const formattedRobux = new Intl.NumberFormat('pt-BR').format(result.robux)

            await interaction.reply({
                embeds: [new EmbedBuilder()
                    .setTitle('Calculadora DevEx')
                    .setDescription(`${formattedRobux} Robux ganhos equivalem aproximadamente a **${formatMoney(result.localAmount, result.currency)}**.`)
                    .addFields(
                        { name: 'Valor em USD', value: formatMoney(result.usdAmount, 'USD'), inline: true },
                        { name: `Valor em ${result.currency}`, value: formatMoney(result.localAmount, result.currency), inline: true },
                        { name: 'Taxa DevEx', value: `US$ ${DEVEX_USD_PER_ROBUX.toFixed(4)} por Robux`, inline: true },
                        { name: 'Cotação utilizada', value: `1 USD = ${result.rate.toLocaleString('pt-BR', { maximumFractionDigits: 6 })} ${result.currency}`, inline: false },
                    )
                    .setFooter({ text: `Cotação de ${result.date}. Estimativa sem impostos, taxas ou retenções.` })
                    .setColor(0x8b5cf6)],
            })
        } catch (error) {
            this.logger.error('Erro ao calcular DevEx', error)
            await interaction.reply({
                content: 'Não foi possível obter a cotação dessa moeda. Use um código ISO 4217 válido, como `BRL`, `USD` ou `EUR`, e tente novamente.',
                flags: MessageFlags.Ephemeral,
            })
        }
    }
}
