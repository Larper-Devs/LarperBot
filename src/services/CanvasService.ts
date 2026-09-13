import { createCanvas, loadImage, type Canvas, type Image } from '@napi-rs/canvas'

export const COLORS = {
    background: '#160a2b',
    backgroundLight: '#2b1454',
    card: '#29134d',
    purple: '#8b5cf6',
    purpleLight: '#c4b5fd',
    white: '#ffffff',
    muted: '#d8ccf5',
    track: '#43276b',
} as const

export type ProfileCardInput = {
    username: string
    tag: string
    avatarUrl: string
    level: number
    totalXp: number
    weeklyXp: number
    monthlyXp: number
    wallet: number
    rank: number
}

export type RankingEntry = {
    position: number
    username: string
    userId: string
    avatarUrl: string
    xp: number
    level: number
}

function roundedRect(ctx: any, x: number, y: number, width: number, height: number, radius: number): void {
    const r = Math.min(radius, width / 2, height / 2)
    ctx.beginPath()
    ctx.moveTo(x + r, y)
    ctx.arcTo(x + width, y, x + width, y + height, r)
    ctx.arcTo(x + width, y + height, x, y + height, r)
    ctx.arcTo(x, y + height, x, y, r)
    ctx.arcTo(x, y, x + width, y, r)
    ctx.closePath()
}

function drawBackground(ctx: any, width: number, height: number): void {
    const gradient = ctx.createLinearGradient(0, 0, width, height)
    gradient.addColorStop(0, COLORS.background)
    gradient.addColorStop(1, COLORS.backgroundLight)
    ctx.fillStyle = gradient
    ctx.fillRect(0, 0, width, height)

    ctx.globalAlpha = 0.18
    ctx.fillStyle = COLORS.purple
    ctx.beginPath()
    ctx.arc(width - 40, 30, 210, 0, Math.PI * 2)
    ctx.fill()
    ctx.globalAlpha = 0.12
    ctx.beginPath()
    ctx.arc(80, height + 30, 180, 0, Math.PI * 2)
    ctx.fill()
    ctx.globalAlpha = 1
}

function drawAvatar(ctx: any, image: Image, x: number, y: number, size: number): void {
    ctx.save()
    ctx.beginPath()
    ctx.arc(x + size / 2, y + size / 2, size / 2, 0, Math.PI * 2)
    ctx.clip()
    ctx.drawImage(image, x, y, size, size)
    ctx.restore()
    ctx.strokeStyle = COLORS.purpleLight
    ctx.lineWidth = 6
    ctx.beginPath()
    ctx.arc(x + size / 2, y + size / 2, size / 2 - 3, 0, Math.PI * 2)
    ctx.stroke()
}

function fitText(ctx: any, text: string, maxWidth: number): string {
    if (ctx.measureText(text).width <= maxWidth) return text
    let result = text
    while (result.length > 3 && ctx.measureText(`${result}...`).width > maxWidth) result = result.slice(0, -1)
    return `${result}...`
}

function progressValues(level: number, xp: number): { current: number, required: number, progress: number } {
    const currentFloor = level ** 2 * 100
    const nextFloor = (level + 1) ** 2 * 100
    const required = Math.max(1, nextFloor - currentFloor)
    return { current: Math.max(0, xp - currentFloor), required, progress: Math.min(1, Math.max(0, (xp - currentFloor) / required)) }
}

export async function renderProfileCard(input: ProfileCardInput): Promise<Buffer> {
    const width = 1000
    const height = 390
    const canvas = createCanvas(width, height)
    const ctx = canvas.getContext('2d')
    drawBackground(ctx, width, height)

    const avatar = await loadImage(input.avatarUrl)
    roundedRect(ctx, 24, 24, width - 48, height - 48, 26)
    ctx.fillStyle = COLORS.card
    ctx.globalAlpha = 0.94
    ctx.fill()
    ctx.globalAlpha = 1

    drawAvatar(ctx, avatar, 58, 75, 190)
    ctx.fillStyle = COLORS.white
    ctx.font = '700 38px sans-serif'
    ctx.fillText(fitText(ctx, input.username, 490), 285, 112)
    ctx.fillStyle = COLORS.muted
    ctx.font = '24px sans-serif'
    ctx.fillText(fitText(ctx, input.tag, 490), 285, 150)
    ctx.fillStyle = COLORS.purpleLight
    ctx.font = '700 25px sans-serif'
    ctx.fillText(`NÍVEL ${input.level}`, 285, 202)
    ctx.fillStyle = COLORS.muted
    ctx.font = '22px sans-serif'
    ctx.fillText(`RANK #${input.rank}`, 450, 202)

    const progress = progressValues(input.level, input.totalXp)
    roundedRect(ctx, 285, 230, 600, 24, 12)
    ctx.fillStyle = COLORS.track
    ctx.fill()
    roundedRect(ctx, 285, 230, Math.max(24, 600 * progress.progress), 24, 12)
    ctx.fillStyle = COLORS.purple
    ctx.fill()
    ctx.fillStyle = COLORS.white
    ctx.font = '18px sans-serif'
    ctx.fillText(`${progress.current}/${progress.required} XP`, 285, 285)

    const stats = [`TOTAL XP  ${input.totalXp}`, `SEMANA  ${input.weeklyXp}`, `MÊS  ${input.monthlyXp}`, `CARTEIRA  ${input.wallet}`]
    stats.forEach((stat, index) => {
        const x = 285 + (index % 2) * 300
        const y = 320 + Math.floor(index / 2) * 28
        ctx.fillStyle = index === 3 ? COLORS.purpleLight : COLORS.muted
        ctx.font = `${index === 3 ? '700' : '400'} 18px sans-serif`
        ctx.fillText(stat, x, y)
    })

    return canvas.encode('png')
}

export async function renderRankingCard(period: string, entries: RankingEntry[]): Promise<Buffer> {
    const width = 1100
    const rowHeight = 58
    const height = Math.max(300, 180 + entries.length * rowHeight)
    const canvas = createCanvas(width, height)
    const ctx = canvas.getContext('2d')
    drawBackground(ctx, width, height)

    ctx.fillStyle = COLORS.white
    ctx.font = '700 38px sans-serif'
    ctx.fillText(`RANKING ${period.toUpperCase()}`, 52, 68)
    ctx.fillStyle = COLORS.muted
    ctx.font = '20px sans-serif'
    ctx.fillText('Os maiores destaques da comunidade', 54, 103)

    for (const entry of entries) {
        const y = 130 + (entry.position - 1) * rowHeight
        roundedRect(ctx, 38, y, width - 76, rowHeight - 8, 16)
        ctx.fillStyle = entry.position <= 3 ? '#3b1c6b' : COLORS.card
        ctx.fill()

        const avatar = await loadImage(entry.avatarUrl)
        drawAvatar(ctx, avatar, 65, y + 6, 40)
        ctx.fillStyle = entry.position === 1 ? '#f7d774' : entry.position === 2 ? '#d9e2f2' : entry.position === 3 ? '#d99a6c' : COLORS.purpleLight
        ctx.font = '700 24px sans-serif'
        ctx.fillText(`#${entry.position}`, 125, y + 34)
        ctx.fillStyle = COLORS.white
        ctx.font = '700 21px sans-serif'
        ctx.fillText(fitText(ctx, entry.username, 430), 210, y + 31)
        ctx.fillStyle = COLORS.muted
        ctx.font = '18px sans-serif'
        ctx.fillText(`Nível ${entry.level}`, 690, y + 31)
        ctx.fillStyle = COLORS.purpleLight
        ctx.font = '700 19px sans-serif'
        ctx.textAlign = 'right'
        ctx.fillText(`${entry.xp} XP`, width - 70, y + 31)
        ctx.textAlign = 'left'
    }

    return canvas.encode('png')
}
