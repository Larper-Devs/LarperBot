import mongoose from 'mongoose'

const { model, models, Schema } = mongoose

export interface RuleItem {
    id: number
    title: string
    description: string
}

export interface RulesConfigData {
    guildId: string
    initialized: boolean
    title: string
    description: string
    color: string
    items: RuleItem[]
    channelId: string | null
    messageId: string | null
}

const rulesConfigSchema = new Schema<RulesConfigData>({
    guildId: { type: String, required: true, unique: true, index: true },
    initialized: { type: Boolean, default: false },
    title: { type: String, default: 'Regras do servidor' },
    description: { type: String, default: 'Leia e respeite as regras para manter a comunidade organizada.' },
    color: { type: String, default: '#8B5CF6' },
    items: {
        type: [{
            id: { type: Number, required: true },
            title: { type: String, required: true },
            description: { type: String, required: true },
        }],
        default: [],
    },
    channelId: { type: String, default: null },
    messageId: { type: String, default: null },
}, { timestamps: true })

export const RulesConfigModel = models.RulesConfig ?? model<RulesConfigData>('RulesConfig', rulesConfigSchema)
