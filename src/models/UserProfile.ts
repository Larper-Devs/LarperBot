import mongoose from 'mongoose'

const { model, models, Schema } = mongoose

export interface InventoryItem {
    itemId: string
    quantity: number
}

export interface UserProfileData {
    guildId: string
    userId: string
    username: string
    wallet: number
    bank: number
    totalXp: number
    weeklyXp: number
    monthlyXp: number
    weekKey: string
    monthKey: string
    lastXpAt: Date | null
    lastDailyAt: Date | null
    lastWorkAt: Date | null
    afkMessage: string | null
    inventory: InventoryItem[]
}

const userProfileSchema = new Schema<UserProfileData>({
    guildId: { type: String, required: true, index: true },
    userId: { type: String, required: true, index: true },
    username: { type: String, default: '' },
    wallet: { type: Number, default: 0, min: 0 },
    bank: { type: Number, default: 0, min: 0 },
    totalXp: { type: Number, default: 0, min: 0 },
    weeklyXp: { type: Number, default: 0, min: 0 },
    monthlyXp: { type: Number, default: 0, min: 0 },
    weekKey: { type: String, default: '' },
    monthKey: { type: String, default: '' },
    lastXpAt: { type: Date, default: null },
    lastDailyAt: { type: Date, default: null },
    lastWorkAt: { type: Date, default: null },
    afkMessage: { type: String, default: null },
    inventory: {
        type: [{ itemId: String, quantity: Number }],
        default: [],
    },
}, { timestamps: true })

userProfileSchema.index({ guildId: 1, userId: 1 }, { unique: true })

export const UserProfileModel = models.UserProfile ?? model<UserProfileData>('UserProfile', userProfileSchema)
