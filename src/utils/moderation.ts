import { GuildMember, PermissionFlagsBits, User } from 'discord.js'
import { ModerationCaseModel, ModerationAction } from '../models/ModerationCase.js'

export function canModerate(actor: GuildMember, target: GuildMember, bot: GuildMember | null): string | null {
    if (actor.id === target.id) return 'Você não pode aplicar essa ação em si mesmo.'
    if (target.id === actor.guild.ownerId) return 'O dono do servidor não pode ser moderado.'
    if (target.id === actor.guild.members.me?.id) return 'Eu não posso aplicar essa ação em mim mesmo.'
    if (target.roles.highest.position >= actor.roles.highest.position && actor.id !== actor.guild.ownerId) {
        return 'O cargo do alvo está acima ou no mesmo nível que o seu.'
    }
    if (bot && target.roles.highest.position >= bot.roles.highest.position) {
        return 'O cargo do alvo está acima ou no mesmo nível que o meu.'
    }
    return null
}

export function hasModerationPermission(member: GuildMember, permission: bigint): boolean {
    return member.permissions.has(permission)
}

export async function resolveMember(guild: GuildMember['guild'], user: User): Promise<GuildMember | null> {
    try {
        return await guild.members.fetch(user.id)
    } catch {
        return null
    }
}

export async function createModerationCase(input: {
    guildId: string
    action: ModerationAction
    targetId: string
    targetTag: string
    moderatorId: string
    reason: string
    durationMs?: number | null
    expiresAt?: Date | null
}) {
    const latest = await ModerationCaseModel.findOne({ guildId: input.guildId }).sort({ caseNumber: -1 }).select('caseNumber').lean()
    return ModerationCaseModel.create({
        ...input,
        caseNumber: (latest?.caseNumber ?? 0) + 1,
        durationMs: input.durationMs ?? null,
        expiresAt: input.expiresAt ?? null,
        active: true,
    })
}

export function permissionForAction(action: ModerationAction): bigint {
    switch (action) {
        case 'warn': return PermissionFlagsBits.ModerateMembers
        case 'timeout':
        case 'untimeout': return PermissionFlagsBits.ModerateMembers
        case 'kick': return PermissionFlagsBits.KickMembers
        case 'ban':
        case 'unban': return PermissionFlagsBits.BanMembers
        case 'clear': return PermissionFlagsBits.ManageMessages
        default: return PermissionFlagsBits.ModerateMembers
    }
}
