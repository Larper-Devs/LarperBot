import { Message, TextChannel } from 'discord.js'
import { GuildConfigModel } from '../models/GuildConfig.js'
import { createModerationCase } from '../utils/moderation.js'
import type { CustomClient } from '../structures/Client.js'

type UserActivity = {
    timestamps: number[]
    contents: string[]
}

const activity = new Map<string, UserActivity>()

function normalize(text: string): string {
    return text.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '')
}

function isExempt(message: Message, config: Awaited<ReturnType<typeof GuildConfigModel.findOne>>): boolean {
    if (!config || !message.member) return false
    if (config.automod.exemptChannelIds.includes(message.channelId)) return true
    return message.member.roles.cache.some((role) => config.automod.exemptRoleIds.includes(role.id))
}

export async function handleMessage(client: CustomClient, message: Message): Promise<boolean> {
    if (!client.automodEnabled || !message.inGuild() || message.author.bot || !client.databaseReady) return false

    const config = await GuildConfigModel.findOne({ guildId: message.guildId })
    if (!config?.automod.enabled || isExempt(message, config)) return false

    const key = `${message.guildId}:${message.author.id}`
    const now = Date.now()
    const state = activity.get(key) ?? { timestamps: [], contents: [] }
    const windowMs = config.automod.spamWindowSeconds * 1_000
    state.timestamps = state.timestamps.filter((timestamp) => now - timestamp <= windowMs)
    state.contents = state.contents.slice(-(config.automod.duplicateLimit - 1))
    state.timestamps.push(now)

    const content = normalize(message.content)
    const violations: string[] = []
    if (state.timestamps.length >= config.automod.spamLimit) violations.push('spam')
    if (state.contents.filter((previous) => previous === content).length + 1 >= config.automod.duplicateLimit) violations.push('mensagens repetidas')
    if (message.mentions.users.size >= config.automod.mentionLimit) violations.push('mention spam')
    if (config.automod.blockedWords.some((word: string) => content.includes(normalize(word)))) violations.push('palavra bloqueada')
    if (config.automod.blockedDomains.some((domain: string) => content.includes(normalize(domain)))) violations.push('link bloqueado')

    const letters = message.content.match(/[A-Za-zÀ-ÿ]/g) ?? []
    const uppercase = letters.filter((letter) => letter === letter.toUpperCase()).length
    if (letters.length >= 12 && (uppercase / letters.length) * 100 >= config.automod.capsPercent) violations.push('caps lock')

    state.contents.push(content)
    activity.set(key, state)
    if (violations.length === 0) return false

    await message.delete().catch(() => undefined)
    const reason = `Automod: ${violations.join(', ')}`
    await createModerationCase({
        guildId: message.guildId,
        action: 'automod',
        targetId: message.author.id,
        targetTag: message.author.tag,
        moderatorId: client.user?.id ?? 'automod',
        reason,
    })

    if (violations.includes('spam') || violations.includes('mention spam')) {
        await message.member?.timeout(60_000, reason).catch(() => undefined)
    }

    const logChannelId = config.logging.automodChannelId
    const logChannel = logChannelId ? message.guild.channels.cache.get(logChannelId) : null
    if (logChannel instanceof TextChannel) {
        await logChannel.send(`Automod — ${message.author.tag} — ${reason}`).catch(() => undefined)
    }

    return true
}
