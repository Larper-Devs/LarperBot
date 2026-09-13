import { randomInt } from 'crypto'
import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags, SlashCommandBuilder } from 'discord.js'
import { creditWallet } from '../../services/EconomyService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'
import { requireDatabase, formatCurrency } from '../../utils/database.js'
import { getProfile } from '../../utils/profiles.js'

const jobs = ['consertou um servidor', 'criou uma arte', 'entregou encomendas', 'organizou um evento', 'programou uma automação']

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'work', description: 'Trabalha para ganhar moedas.', category: 'Economia', data: new SlashCommandBuilder().setName('work').setDescription('Trabalha para ganhar moedas.') })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.guild || !await requireDatabase(client, interaction)) return
        const profile = await getProfile(interaction.guild.id, interaction.user.id, interaction.user.username)
        const cooldown = 60 * 60 * 1_000
        if (profile.lastWorkAt && Date.now() - profile.lastWorkAt.getTime() < cooldown) {
            await interaction.reply({ content: `Você já trabalhou recentemente. Tente novamente <t:${Math.floor((profile.lastWorkAt.getTime() + cooldown) / 1000)}:R>.`, flags: MessageFlags.Ephemeral })
            return
        }
        const amount = randomInt(100, 301)
        profile.lastWorkAt = new Date()
        await profile.save()
        await creditWallet(interaction.guild.id, interaction.user.id, amount, 'Recompensa de trabalho', 'reward')
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Trabalho concluído').setDescription(`Você ${jobs[randomInt(0, jobs.length)]} e recebeu **${formatCurrency(amount)}**.`).setColor(0x2ecc71)] })
    }
}
