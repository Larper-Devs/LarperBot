import { UserProfileModel } from '../models/UserProfile.js'
import { EconomyTransactionModel, EconomyTransactionType } from '../models/EconomyTransaction.js'
import { getProfile } from '../utils/profiles.js'

export const SHOP_ITEMS = {
    coffee: { name: 'Café', price: 50, description: 'Um café virtual para o inventário.' },
    cookie: { name: 'Cookie', price: 100, description: 'Um cookie virtual para presentear.' },
    lucky: { name: 'Amuleto da sorte', price: 500, description: 'Item raro da loja.' },
} as const

export async function recordTransaction(input: {
    guildId: string
    userId: string
    type: EconomyTransactionType
    amount: number
    reason: string
    fromUserId?: string | null
    toUserId?: string | null
    idempotencyKey?: string | null
}) {
    return EconomyTransactionModel.create({
        ...input,
        fromUserId: input.fromUserId ?? null,
        toUserId: input.toUserId ?? null,
        idempotencyKey: input.idempotencyKey ?? null,
    })
}

export async function creditWallet(guildId: string, userId: string, amount: number, reason: string, type: EconomyTransactionType = 'credit') {
    if (amount <= 0) throw new Error('O valor deve ser positivo.')
    const profile = await UserProfileModel.findOneAndUpdate(
        { guildId, userId },
        { $inc: { wallet: amount } },
        { returnDocument: 'after', upsert: true, setDefaultsOnInsert: true },
    )
    await recordTransaction({ guildId, userId, type, amount, reason })
    return profile
}

export async function debitWallet(guildId: string, userId: string, amount: number, reason: string, type: EconomyTransactionType = 'debit') {
    if (amount <= 0) throw new Error('O valor deve ser positivo.')
    const profile = await UserProfileModel.findOneAndUpdate(
        { guildId, userId, wallet: { $gte: amount } },
        { $inc: { wallet: -amount } },
        { returnDocument: 'after' },
    )
    if (!profile) throw new Error('Saldo insuficiente.')
    await recordTransaction({ guildId, userId, type, amount, reason })
    return profile
}

export async function transferWallet(guildId: string, fromUserId: string, toUserId: string, amount: number) {
    if (fromUserId === toUserId) throw new Error('Você não pode transferir para si mesmo.')
    await debitWallet(guildId, fromUserId, amount, `Transferência para ${toUserId}`, 'transfer')
    try {
        await creditWallet(guildId, toUserId, amount, `Transferência de ${fromUserId}`, 'transfer')
    } catch (error) {
        await creditWallet(guildId, fromUserId, amount, 'Estorno de transferência', 'reward')
        throw error
    }
}

export async function buyItem(guildId: string, userId: string, itemId: keyof typeof SHOP_ITEMS) {
    const item = SHOP_ITEMS[itemId]
    await debitWallet(guildId, userId, item.price, `Compra: ${item.name}`, 'purchase')
    const profile = await getProfile(guildId, userId)
    const inventoryItem = profile.inventory.find((entry: { itemId: string, quantity: number }) => entry.itemId === itemId)
    if (inventoryItem) inventoryItem.quantity += 1
    else profile.inventory.push({ itemId, quantity: 1 })
    await profile.save()
    return item
}
