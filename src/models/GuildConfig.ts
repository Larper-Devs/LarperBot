import mongoose from 'mongoose'

const { model, models, Schema } = mongoose

export interface GuildConfigData {
    guildId: string
    logging: {
        moderationChannelId: string | null
        automodChannelId: string | null
    }
    automod: {
        enabled: boolean
        spamLimit: number
        spamWindowSeconds: number
        duplicateLimit: number
        mentionLimit: number
        capsPercent: number
        blockedWords: string[]
        blockedDomains: string[]
        exemptRoleIds: string[]
        exemptChannelIds: string[]
    }
    welcome: {
        enabled: boolean
        channelId: string | null
        message: string
        embedEnabled: boolean
        embedTitle: string
        embedDescription: string
        embedColor: string
        imageUrl: string | null
        leaveEnabled: boolean
        leaveChannelId: string | null
        leaveMessage: string
        autoRoleId: string | null
        colorRoleIds: string[]
        panelChannelId: string | null
        panelMessageId: string | null
    }
    level: {
        enabled: boolean
        xpCooldownSeconds: number
        xpMin: number
        xpMax: number
        ignoredChannelIds: string[]
        announceChannelId: string | null
    }
}

const guildConfigSchema = new Schema<GuildConfigData>({
    guildId: { type: String, required: true, unique: true, index: true },
    logging: {
        moderationChannelId: { type: String, default: null },
        automodChannelId: { type: String, default: null },
    },
    automod: {
        enabled: { type: Boolean, default: false },
        spamLimit: { type: Number, default: 6 },
        spamWindowSeconds: { type: Number, default: 8 },
        duplicateLimit: { type: Number, default: 3 },
        mentionLimit: { type: Number, default: 5 },
        capsPercent: { type: Number, default: 75 },
        blockedWords: { type: [String], default: [] },
        blockedDomains: { type: [String], default: [] },
        exemptRoleIds: { type: [String], default: [] },
        exemptChannelIds: { type: [String], default: [] },
    },
    welcome: {
        enabled: { type: Boolean, default: false },
        channelId: { type: String, default: null },
        message: { type: String, default: 'Bem-vindo(a), {mention}!' },
        embedEnabled: { type: Boolean, default: true },
        embedTitle: { type: String, default: 'Bem-vindo(a)!' },
        embedDescription: { type: String, default: '{message}' },
        embedColor: { type: String, default: '#5865F2' },
        imageUrl: { type: String, default: null },
        leaveEnabled: { type: Boolean, default: false },
        leaveChannelId: { type: String, default: null },
        leaveMessage: { type: String, default: '{username} saiu do servidor.' },
        autoRoleId: { type: String, default: null },
        colorRoleIds: { type: [String], default: [] },
        panelChannelId: { type: String, default: null },
        panelMessageId: { type: String, default: null },
    },
    level: {
        enabled: { type: Boolean, default: false },
        xpCooldownSeconds: { type: Number, default: 60 },
        xpMin: { type: Number, default: 15 },
        xpMax: { type: Number, default: 25 },
        ignoredChannelIds: { type: [String], default: [] },
        announceChannelId: { type: String, default: null },
    },
}, { timestamps: true })

export const GuildConfigModel = models.GuildConfig ?? model<GuildConfigData>('GuildConfig', guildConfigSchema)
