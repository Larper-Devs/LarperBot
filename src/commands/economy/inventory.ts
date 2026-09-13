import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { SHOP_ITEMS } from '../../services/EconomyService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase } from '../../utils/database.js'
import { getProfile } from '../../utils/profiles.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'inventory', description: 'Consulta o inventário de um usuário.', category: 'Economia', data: new SlashCommandBuilder().setName('inventory').setDescription('Consulta o inventário de um usuário.').addUserOption((option) => option.setName('usuario').setDescription('Usuário consultado.').setRequired(false)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        const user = interaction.options.getUser('usuario') ?? interaction.user
        const profile = await getProfile(interaction.guild.id, user.id, user.username)
        const description = profile.inventory.length === 0 ? 'Inventário vazio.' : profile.inventory.map((entry: { itemId: string, quantity: number }) => `${SHOP_ITEMS[entry.itemId as keyof typeof SHOP_ITEMS]?.name ?? entry.itemId}: **${entry.quantity}**`).join('\n')
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle(`Inventário de ${user.username}`).setDescription(description).setColor(0x5865f2)] })
    }
}
