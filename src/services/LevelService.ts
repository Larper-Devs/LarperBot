import { randomInt } from 'crypto'
import { Message, TextChannel } from 'discord.js'
import { GuildConfigModel } from '../models/GuildConfig.js'
import { UserProfileModel } from '../models/UserProfile.js'
import { getProfile } from '../utils/profiles.js'
import type { CustomClient } from '../structures/Client.js'

export function levelFromXp(xp: number): number {
    return Math.floor(Math.sqrt(Math.max(0, xp) / 100))
}

export async function processMessage(client: CustomClient, message: Message): Promise<void> {
    if (!client.levelsEnabled || !client.databaseReady || !message.inGuild() || message.author.bot) return

    const config = await GuildConfigModel.findOne({ guildId: message.guildId })
    if (!config?.level.enabled || config.level.ignoredChannelIds.includes(message.channelId)) return

    const profile = await getProfile(message.guildId, message.author.id, message.author.username)
    if (profile.lastXpAt && Date.now() - profile.lastXpAt.getTime() < config.level.xpCooldownSeconds * 1_000) return

    const amount = randomInt(config.level.xpMin, Math.max(config.level.xpMin + 1, config.level.xpMax + 1))
    const oldLevel = levelFromXp(profile.totalXp)
    profile.totalXp += amount
    profile.weeklyXp += amount
    profile.monthlyXp += amount
    profile.lastXpAt = new Date()
    await profile.save()

    const newLevel = levelFromXp(profile.totalXp)
    if (newLevel <= oldLevel || !config.level.announceChannelId) return

    const channel = message.guild.channels.cache.get(config.level.announceChannelId)
    if (channel instanceof TextChannel) {
        await channel.send(`🎉 ${message.author} chegou ao nível **${newLevel}**!`).catch(() => undefined)
    }
}

export async function getLeaderboard(guildId: string, period: 'total' | 'weekly' | 'monthly', limit = 10) {
    const field = period === 'total' ? 'totalXp' : period === 'weekly' ? 'weeklyXp' : 'monthlyXp'
    return UserProfileModel.find({ guildId }).sort({ [field]: -1 }).limit(limit).lean()
}
