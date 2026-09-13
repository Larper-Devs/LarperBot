import { EmbedBuilder, TextChannel } from 'discord.js'
import { RulesConfigModel } from '../models/RulesConfig.js'
import type { RuleItem, RulesConfigData } from '../models/RulesConfig.js'
import type { CustomClient } from '../structures/Client.js'

export const DEFAULT_RULES: RuleItem[] = [
    { id: 1, title: 'Respeito', description: 'Trate todos os membros com respeito. Não serão tolerados ataques pessoais, preconceito ou assédio.' },
    { id: 2, title: 'Conteúdo adequado', description: 'Não envie conteúdo ilegal, NSFW, chocante ou que viole as regras do Discord.' },
    { id: 3, title: 'Sem spam', description: 'Evite flood, mensagens repetidas, spam de menções e divulgação não autorizada.' },
    { id: 4, title: 'Canais corretos', description: 'Use cada canal para o assunto indicado e mantenha as conversas organizadas.' },
    { id: 5, title: 'Divulgação', description: 'Divulgações, links e convites só são permitidos quando autorizados pela equipe.' },
    { id: 6, title: 'Privacidade', description: 'Nunca compartilhe dados pessoais seus ou de outras pessoas.' },
    { id: 7, title: 'Nomes e avatares', description: 'Nomes, avatares e status não podem ser ofensivos, enganadores ou impróprios.' },
    { id: 8, title: 'Equipe', description: 'Siga as orientações da equipe e utilize os canais adequados para denúncias e dúvidas.' },
]

export async function getRulesConfig(guildId: string) {
    const config = await RulesConfigModel.findOneAndUpdate(
        { guildId },
        { $setOnInsert: { guildId } },
        { returnDocument: 'after', upsert: true, setDefaultsOnInsert: true },
    )
    if (!config) throw new Error('Não foi possível carregar as regras.')

    if (!config.initialized) {
        config.items = DEFAULT_RULES.map((rule) => ({ ...rule }))
        config.initialized = true
        await config.save()
    }

    return config
}

export function rulesEmbed(config: Pick<RulesConfigData, 'title' | 'description' | 'color' | 'items'>): EmbedBuilder {
    const embed = new EmbedBuilder()
        .setTitle(config.title)
        .setDescription(config.description)
        .setColor(config.color as `#${string}`)
        .setFooter({ text: 'Leia as regras antes de participar.' })

    if (config.items.length === 0) {
        embed.addFields({ name: 'Nenhuma regra cadastrada', value: 'A equipe ainda não adicionou regras.' })
    } else {
        embed.addFields(config.items.slice(0, 25).map((rule) => ({
            name: `${rule.id}. ${rule.title}`,
            value: rule.description,
            inline: false,
        })))
    }

    return embed
}

export async function syncRulesMessage(client: CustomClient, config: RulesConfigData): Promise<void> {
    if (!config.channelId || !config.messageId) return

    const channel = await client.channels.fetch(config.channelId).catch(() => null)
    if (!(channel instanceof TextChannel)) return

    const message = await channel.messages.fetch(config.messageId).catch(() => null)
    if (!message) return
    await message.edit({ embeds: [rulesEmbed(config)] }).catch(() => undefined)
}
