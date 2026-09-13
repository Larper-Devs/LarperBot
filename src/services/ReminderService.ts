import { Client, TextChannel } from 'discord.js'
import { ReminderModel } from '../models/Reminder.js'
import type { CustomClient } from '../structures/Client.js'

let timer: NodeJS.Timeout | null = null

export function startReminderScheduler(client: CustomClient): void {
    if (timer) return
    timer = setInterval(async () => {
        if (!client.databaseReady) return
        const reminders = await ReminderModel.find({ sent: false, remindAt: { $lte: new Date() } }).limit(25)
        for (const reminder of reminders) {
            reminder.sent = true
            await reminder.save()
            const channel = await client.channels.fetch(reminder.channelId).catch(() => null)
            if (channel instanceof TextChannel) {
                await channel.send(`<@${reminder.userId}> ⏰ Lembrete: ${reminder.message}`).catch(() => undefined)
            }
        }
    }, 15_000)
}
