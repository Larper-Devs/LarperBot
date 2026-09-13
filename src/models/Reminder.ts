import mongoose from 'mongoose'

const { model, models, Schema } = mongoose

export interface ReminderData {
    guildId: string
    userId: string
    channelId: string
    message: string
    remindAt: Date
    sent: boolean
}

const reminderSchema = new Schema<ReminderData>({
    guildId: { type: String, required: true, index: true },
    userId: { type: String, required: true, index: true },
    channelId: { type: String, required: true },
    message: { type: String, required: true, maxlength: 1000 },
    remindAt: { type: Date, required: true, index: true },
    sent: { type: Boolean, default: false, index: true },
}, { timestamps: true })

export const ReminderModel = models.Reminder ?? model<ReminderData>('Reminder', reminderSchema)
