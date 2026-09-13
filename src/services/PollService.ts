import { randomUUID } from 'crypto'
import { ActionRowBuilder, ButtonBuilder, ButtonInteraction, ButtonStyle, EmbedBuilder, ChatInputCommandInteraction, MessageFlags } from 'discord.js'

type Poll = {
    id: string
    ownerId: string
    question: string
    options: string[]
    votes: Map<string, number>
}

const polls = new Map<string, Poll>()

function render(poll: Poll): EmbedBuilder {
    const counts = poll.options.map((_, index) => [...poll.votes.values()].filter((vote) => vote === index).length)
    const total = counts.reduce((sum, count) => sum + count, 0)
    const description = poll.options.map((option, index) => `**${index + 1}. ${option}** — ${counts[index]} voto(s)`).join('\n')
    return new EmbedBuilder().setTitle(`Enquete: ${poll.question}`).setDescription(`${description}\n\nTotal: ${total} voto(s)`).setColor(0x5865f2)
}

export async function createPoll(interaction: ChatInputCommandInteraction, question: string, options: string[]): Promise<void> {
    const id = randomUUID().slice(0, 8)
    const poll: Poll = { id, ownerId: interaction.user.id, question, options, votes: new Map() }
    polls.set(id, poll)
    const row = new ActionRowBuilder<ButtonBuilder>().addComponents(options.map((option, index) => new ButtonBuilder().setCustomId(`poll:${id}:${index}`).setLabel(option.slice(0, 80)).setStyle(ButtonStyle.Primary)))
    await interaction.reply({ embeds: [render(poll)], components: [row] })
}

export async function handlePollInteraction(interaction: ButtonInteraction): Promise<boolean> {
    if (!interaction.customId.startsWith('poll:')) return false
    const [, id, indexValue] = interaction.customId.split(':')
    const poll = polls.get(id)
    if (!poll) {
        await interaction.reply({ content: 'Essa enquete expirou.', flags: MessageFlags.Ephemeral })
        return true
    }
    const index = Number(indexValue)
    if (!Number.isInteger(index) || index < 0 || index >= poll.options.length) return true
    poll.votes.set(interaction.user.id, index)
    await interaction.update({ embeds: [render(poll)] })
    return true
}
