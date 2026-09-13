import { UserProfileModel } from '../models/UserProfile.js'
import { currentMonthKey, currentWeekKey } from './database.js'

export async function getProfile(guildId: string, userId: string, username = '') {
    const weekKey = currentWeekKey()
    const monthKey = currentMonthKey()
    const profile = await UserProfileModel.findOneAndUpdate(
        { guildId, userId },
        {
            $set: { username },
            $setOnInsert: { weekKey, monthKey },
        },
        { returnDocument: 'after', upsert: true, setDefaultsOnInsert: true },
    )

    if (!profile) throw new Error('Não foi possível carregar o perfil.')

    let changed = false
    if (profile.weekKey !== weekKey) {
        profile.weekKey = weekKey
        profile.weeklyXp = 0
        changed = true
    }
    if (profile.monthKey !== monthKey) {
        profile.monthKey = monthKey
        profile.monthlyXp = 0
        changed = true
    }
    if (changed) await profile.save()

    return profile
}
