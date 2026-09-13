import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { buyItem, SHOP_ITEMS } from '../../services/EconomyService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase, formatCurrency } from '../../utils/database.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'shop',
            description: 'Consulta ou compra itens da loja.',
            category: 'Economia',
            data: new SlashCommandBuilder()
                .setName('shop')
                .setDescription('Consulta ou compra itens da loja.')
                .addSubcommand((subcommand) => subcommand.setName('list').setDescription('Lista os itens disponíveis.'))
                .addSubcommand((subcommand) => subcommand
                    .setName('buy')
                    .setDescription('Compra um item.')
                    .addStringOption((option) => option
                        .setName('item')
                        .setDescription('Item comprado.')
                        .addChoices(...Object.entries(SHOP_ITEMS).map(([value, item]) => ({ name: `${item.name} — ${item.price}`, value })))
                        .setRequired(true))),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        if (interaction.options.getSubcommand() === 'list') {
            const description = Object.entries(SHOP_ITEMS).map(([id, item]) => `**${item.name}** — \`${id}\` — ${formatCurrency(item.price)}\n${item.description}`).join('\n\n')
            await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Loja').setDescription(description).setColor(0xf1c40f)] })
            return
        }
        const itemId = interaction.options.getString('item', true) as keyof typeof SHOP_ITEMS
        try {
            const item = await buyItem(interaction.guild.id, interaction.user.id, itemId)
            await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Compra realizada').setDescription(`Você comprou **${item.name}**.`).setColor(0x2ecc71)] })
        } catch (error) {
            await interaction.reply({ content: error instanceof Error ? error.message : 'Não foi possível realizar a compra.', flags: MessageFlags.Ephemeral })
        }
    }
}
