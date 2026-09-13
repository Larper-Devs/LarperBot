import { randomInt } from 'crypto'
import { ActionRowBuilder, ButtonBuilder, ButtonInteraction, ButtonStyle, ChatInputCommandInteraction, EmbedBuilder, MessageFlags } from 'discord.js'
import { creditWallet, debitWallet } from './EconomyService.js'

type Card = { value: number, label: string }
type BlackjackGame = { guildId: string, userId: string, bet: number, deck: Card[], player: Card[], dealer: Card[] }
const games = new Map<string, BlackjackGame>()

function deck(): Card[] {
    const cards: Card[] = []
    const labels = ['A', '2', '3', '4', '5', '6', '7', '8', '9', '10', 'J', 'Q', 'K']
    for (let suit = 0; suit < 4; suit++) for (let index = 0; index < labels.length; index++) cards.push({ value: index === 0 ? 11 : Math.min(index + 1, 10), label: labels[index] })
    for (let index = cards.length - 1; index > 0; index--) {
        const swap = randomInt(index + 1)
        ;[cards[index], cards[swap]] = [cards[swap], cards[index]]
    }
    return cards
}

function score(cards: Card[]): number {
    let result = cards.reduce((sum, card) => sum + card.value, 0)
    let aces = cards.filter((card) => card.value === 11).length
    while (result > 21 && aces > 0) { result -= 10; aces-- }
    return result
}

function draw(game: BlackjackGame): Card {
    const card = game.deck.pop()
    if (!card) throw new Error('Baralho vazio.')
    return card
}

function row(game: BlackjackGame, disabled = false): ActionRowBuilder<ButtonBuilder> {
    return new ActionRowBuilder<ButtonBuilder>().addComponents(
        new ButtonBuilder().setCustomId(`blackjack:hit:${game.userId}`).setLabel('Comprar').setStyle(ButtonStyle.Primary).setDisabled(disabled),
        new ButtonBuilder().setCustomId(`blackjack:stand:${game.userId}`).setLabel('Parar').setStyle(ButtonStyle.Success).setDisabled(disabled),
    )
}

function embed(game: BlackjackGame, revealDealer = false, result = ''): EmbedBuilder {
    const dealerCards = revealDealer ? game.dealer.map((card) => card.label).join(' ') : `${game.dealer[0]?.label ?? '?'} 🂠`
    return new EmbedBuilder().setTitle('Blackjack').setDescription(`**Dealer:** ${dealerCards} (${revealDealer ? score(game.dealer) : '?'})\n**Você:** ${game.player.map((card) => card.label).join(' ')} (${score(game.player)})\n\n${result || 'Escolha sua jogada.'}`).setColor(result.includes('venceu') ? 0x2ecc71 : result.includes('perdeu') ? 0xed4245 : 0x5865f2)
}

async function settle(game: BlackjackGame, result: 'win' | 'lose' | 'tie') {
    games.delete(`${game.guildId}:${game.userId}`)
    if (result === 'win') await creditWallet(game.guildId, game.userId, game.bet * 2, 'Vitória no blackjack', 'game')
    if (result === 'tie') await creditWallet(game.guildId, game.userId, game.bet, 'Empate no blackjack', 'game')
}

export async function startBlackjack(interaction: ChatInputCommandInteraction, bet: number): Promise<void> {
    if (!interaction.guild) return
    const key = `${interaction.guild.id}:${interaction.user.id}`
    if (games.has(key)) {
        await interaction.reply({ content: 'Você já possui uma partida em andamento.', flags: MessageFlags.Ephemeral })
        return
    }
    await debitWallet(interaction.guild.id, interaction.user.id, bet, 'Aposta no blackjack', 'game')
    const game: BlackjackGame = { guildId: interaction.guild.id, userId: interaction.user.id, bet, deck: deck(), player: [], dealer: [] }
    game.player.push(draw(game), draw(game))
    game.dealer.push(draw(game), draw(game))
    games.set(key, game)
    await interaction.reply({ embeds: [embed(game)], components: [row(game)] })
}

export async function handleBlackjackInteraction(interaction: ButtonInteraction): Promise<boolean> {
    if (!interaction.customId.startsWith('blackjack:')) return false
    const [, action, userId] = interaction.customId.split(':')
    if (interaction.user.id !== userId) {
        await interaction.reply({ content: 'Essa partida pertence a outro jogador.', flags: MessageFlags.Ephemeral })
        return true
    }
    const key = `${interaction.guildId}:${userId}`
    const game = games.get(key)
    if (!game) {
        await interaction.reply({ content: 'Essa partida expirou.', flags: MessageFlags.Ephemeral })
        return true
    }

    if (action === 'hit') {
        game.player.push(draw(game))
        if (score(game.player) > 21) {
            await settle(game, 'lose')
            await interaction.update({ embeds: [embed(game, true, 'Você perdeu: estourou 21.')], components: [row(game, true)] })
        } else {
            await interaction.update({ embeds: [embed(game)], components: [row(game)] })
        }
        return true
    }

    while (score(game.dealer) < 17) game.dealer.push(draw(game))
    const playerScore = score(game.player)
    const dealerScore = score(game.dealer)
    const result = playerScore > dealerScore || dealerScore > 21 ? 'win' : playerScore === dealerScore ? 'tie' : 'lose'
    await settle(game, result)
    const text = result === 'win' ? `Você venceu e recebeu ${game.bet * 2} moedas.` : result === 'tie' ? 'Empate: sua aposta foi devolvida.' : 'Você perdeu a rodada.'
    await interaction.update({ embeds: [embed(game, true, text)], components: [row(game, true)] })
    return true
}

export async function coinflip(interaction: ChatInputCommandInteraction, bet: number, choice: 'cara' | 'coroa'): Promise<void> {
    if (!interaction.guild) return
    await debitWallet(interaction.guild.id, interaction.user.id, bet, 'Aposta em cara ou coroa', 'game')
    const result = randomInt(0, 2) === 0 ? 'cara' : 'coroa'
    const won = result === choice
    if (won) await creditWallet(interaction.guild.id, interaction.user.id, bet * 2, 'Vitória em cara ou coroa', 'game')
    await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Cara ou coroa').setDescription(`Resultado: **${result}**\n${won ? `Você venceu e ganhou ${bet} moedas de lucro.` : 'Você perdeu a aposta.'}`).setColor(won ? 0x2ecc71 : 0xed4245)] })
}
