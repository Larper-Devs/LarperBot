import mongoose from 'mongoose'

const { model, models, Schema } = mongoose

export type ModerationAction = 'warn' | 'timeout' | 'untimeout' | 'kick' | 'ban' | 'unban' | 'clear' | 'automod'

export interface ModerationCaseData {
    guildId: string
    caseNumber: number
    action: ModerationAction
    targetId: string
    targetTag: string
    moderatorId: string
    reason: string
    durationMs: number | null
    active: boolean
    expiresAt: Date | null
}

const moderationCaseSchema = new Schema<ModerationCaseData>({
    guildId: { type: String, required: true, index: true },
    caseNumber: { type: Number, required: true },
    action: { type: String, required: true },
    targetId: { type: String, required: true, index: true },
    targetTag: { type: String, default: '' },
    moderatorId: { type: String, required: true },
    reason: { type: String, default: 'Sem motivo informado' },
    durationMs: { type: Number, default: null },
    active: { type: Boolean, default: true },
    expiresAt: { type: Date, default: null },
}, { timestamps: true })

moderationCaseSchema.index({ guildId: 1, caseNumber: 1 }, { unique: true })
moderationCaseSchema.index({ expiresAt: 1 }, { expireAfterSeconds: 0, partialFilterExpression: { action: 'timeout', active: false } })

export const ModerationCaseModel = models.ModerationCase ?? model<ModerationCaseData>('ModerationCase', moderationCaseSchema)
