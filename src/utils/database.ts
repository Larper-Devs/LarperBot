import { ChatInputCommandInteraction, EmbedBuilder, MessageFlags } from 'discord.js'
import type { CustomClient } from '../structures/Client.js'

export async function requireDatabase(
    client: CustomClient,
    interaction: ChatInputCommandInteraction,
): Promise<boolean> {
    if (client.databaseReady) return true

    const embed = new EmbedBuilder()
        .setTitle('Persistência indisponível')
        .setDescription('O banco de dados não está conectado. Tente novamente depois que o administrador corrigir o MongoDB.')
        .setColor(0xed4245)

    if (interaction.deferred) {
        await interaction.editReply({ embeds: [embed] })
    } else if (interaction.replied) {
        await interaction.followUp({ embeds: [embed], flags: MessageFlags.Ephemeral })
    } else {
        await interaction.reply({ embeds: [embed], flags: MessageFlags.Ephemeral })
    }

    return false
}

export function currentWeekKey(date = new Date()): string {
    const year = date.getUTCFullYear()
    const firstDay = new Date(Date.UTC(year, 0, 1))
    const dayOfYear = Math.floor((date.getTime() - firstDay.getTime()) / 86_400_000) + 1
    const week = Math.ceil((dayOfYear + firstDay.getUTCDay()) / 7)
    return `${year}-W${String(week).padStart(2, '0')}`
}

export function currentMonthKey(date = new Date()): string {
    return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, '0')}`
}

export function formatCurrency(amount: number): string {
    return `${amount.toLocaleString('pt-BR')} moedas`
}

export function parseDuration(input: string): number | null {
    const match = input.trim().toLowerCase().match(/^(\d+)\s*(s|m|h|d|w)$/)
    if (!match) return null

    const value = Number(match[1])
    const multipliers: Record<string, number> = {
        s: 1_000,
        m: 60_000,
        h: 3_600_000,
        d: 86_400_000,
        w: 604_800_000,
    }

    const duration = value * multipliers[match[2]]
    return duration > 0 && duration <= 28 * 86_400_000 ? duration : null
}

export function replacePlaceholders(template: string, values: Record<string, string>): string {
    return template.replace(/\{(\w+)\}/g, (_, key: string) => values[key] ?? `{${key}}`)
}
