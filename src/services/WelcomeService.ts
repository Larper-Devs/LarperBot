import {
    ActionRowBuilder,
    EmbedBuilder,
    GuildMember,
    MessageFlags,
    StringSelectMenuInteraction,
    StringSelectMenuBuilder,
    TextChannel,
} from 'discord.js'
import { GuildConfigModel } from '../models/GuildConfig.js'
import { replacePlaceholders } from '../utils/database.js'
import type { CustomClient } from '../structures/Client.js'

function values(member: GuildMember) {
    return {
        user: member.user.username,
        username: member.user.username,
        mention: `<@${member.id}>`,
        server: member.guild.name,
        memberCount: String(member.guild.memberCount),
    }
}

export async function sendWelcome(client: CustomClient, member: GuildMember): Promise<void> {
    if (!client.memberEventsEnabled || !client.databaseReady) return
    const config = await GuildConfigModel.findOne({ guildId: member.guild.id })
    if (!config?.welcome.enabled || !config.welcome.channelId) return

    const channel = member.guild.channels.cache.get(config.welcome.channelId)
    if (!(channel instanceof TextChannel)) return

    const data = values(member)
    const message = replacePlaceholders(config.welcome.message, data)
    if (!config.welcome.embedEnabled) {
        await channel.send(message).catch(() => undefined)
    } else {
        const embed = new EmbedBuilder()
            .setTitle(replacePlaceholders(config.welcome.embedTitle, data))
            .setDescription(replacePlaceholders(config.welcome.embedDescription, { ...data, message }))
            .setColor(config.welcome.embedColor as `#${string}`)
            .setThumbnail(member.user.displayAvatarURL())
        if (config.welcome.imageUrl) embed.setImage(config.welcome.imageUrl)
        await channel.send({ content: message, embeds: [embed] }).catch(() => undefined)
    }

    if (config.welcome.autoRoleId) {
        await member.roles.add(config.welcome.autoRoleId, 'Cargo automático de boas-vindas').catch(() => undefined)
    }
}

export async function sendLeave(client: CustomClient, member: GuildMember): Promise<void> {
    if (!client.memberEventsEnabled || !client.databaseReady) return
    const config = await GuildConfigModel.findOne({ guildId: member.guild.id })
    if (!config?.welcome.leaveEnabled || !config.welcome.leaveChannelId) return
    const channel = member.guild.channels.cache.get(config.welcome.leaveChannelId)
    if (!(channel instanceof TextChannel)) return
    await channel.send(replacePlaceholders(config.welcome.leaveMessage, values(member))).catch(() => undefined)
}

export function colorPanel(config: { colorRoleIds: string[] }): ActionRowBuilder<StringSelectMenuBuilder> {
    return new ActionRowBuilder<StringSelectMenuBuilder>().addComponents(
        new StringSelectMenuBuilder()
            .setCustomId('welcome:colors')
            .setPlaceholder('Escolha uma cor')
            .setMinValues(0)
            .setMaxValues(1)
            .addOptions(config.colorRoleIds.map((id) => ({ label: `Cor ${id}`, value: id }))),
    )
}

export async function handleColorSelection(interaction: StringSelectMenuInteraction): Promise<boolean> {
    if (interaction.customId !== 'welcome:colors') return false
    if (!interaction.guild) return true
    const config = await GuildConfigModel.findOne({ guildId: interaction.guild.id })
    if (!config || config.welcome.colorRoleIds.length === 0) {
        await interaction.reply({ content: 'O painel de cores ainda não foi configurado.', flags: MessageFlags.Ephemeral })
        return true
    }
    const member = await interaction.guild.members.fetch(interaction.user.id)
    const selected = interaction.values[0]
    const managed = config.welcome.colorRoleIds.filter((id: string) => id !== selected)
    await member.roles.remove(managed, 'Troca de cor pelo painel').catch(() => undefined)
    if (selected) await member.roles.add(selected, 'Cor escolhida pelo painel').catch(() => undefined)
    await interaction.reply({ content: selected ? 'Sua cor foi atualizada.' : 'Suas cores foram removidas.', flags: MessageFlags.Ephemeral })
    return true
}
