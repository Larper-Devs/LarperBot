import mongoose from 'mongoose'

const { model, models, Schema } = mongoose

export type EconomyTransactionType = 'credit' | 'debit' | 'transfer' | 'reward' | 'game' | 'purchase'

export interface EconomyTransactionData {
    guildId: string
    userId: string
    fromUserId: string | null
    toUserId: string | null
    type: EconomyTransactionType
    amount: number
    reason: string
    idempotencyKey: string | null
}

const economyTransactionSchema = new Schema<EconomyTransactionData>({
    guildId: { type: String, required: true, index: true },
    userId: { type: String, required: true, index: true },
    fromUserId: { type: String, default: null },
    toUserId: { type: String, default: null },
    type: { type: String, required: true },
    amount: { type: Number, required: true, min: 0 },
    reason: { type: String, default: '' },
    idempotencyKey: { type: String, default: null },
}, { timestamps: true })

economyTransactionSchema.index({ guildId: 1, idempotencyKey: 1 }, { unique: true, sparse: true })

export const EconomyTransactionModel = models.EconomyTransaction ?? model<EconomyTransactionData>('EconomyTransaction', economyTransactionSchema)
