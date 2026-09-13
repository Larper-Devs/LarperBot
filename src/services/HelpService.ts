import {
    ActionRowBuilder,
    ComponentType,
    ContainerComponentData,
    MessageFlags,
    PermissionFlagsBits,
    StringSelectMenuBuilder,
    StringSelectMenuInteraction,
} from 'discord.js'
import type { CustomClient } from '../structures/Client.js'

export const HELP_CATEGORY_CUSTOM_ID = 'help:category'

function visibleCommands(client: CustomClient, canSeeAdminCommands: boolean) {
    return client.commandList.filter((command) => canSeeAdminCommands || !command.adminOnly)
}

function categories(client: CustomClient, canSeeAdminCommands: boolean): Map<string, string[]> {
    const grouped = new Map<string, string[]>()
    for (const command of visibleCommands(client, canSeeAdminCommands)) {
        const commands = grouped.get(command.category) ?? []
        commands.push(command.name)
        grouped.set(command.category, commands)
    }

    return new Map([...grouped.entries()].sort(([left], [right]) => left.localeCompare(right, 'pt-BR')))
}

export function helpMenu(
    client: CustomClient,
    ownerId: string,
    canSeeAdminCommands: boolean,
    selectedCategory?: string,
): ActionRowBuilder<StringSelectMenuBuilder> {
    const options = [...categories(client, canSeeAdminCommands).entries()].slice(0, 25).map(([category, commands]) => ({
        label: category.slice(0, 100),
        value: category,
        description: `${commands.length} comando(s)`.slice(0, 100),
        default: category === selectedCategory,
    }))

    return new ActionRowBuilder<StringSelectMenuBuilder>().addComponents(
        new StringSelectMenuBuilder()
            .setCustomId(`${HELP_CATEGORY_CUSTOM_ID}:${ownerId}`)
            .setPlaceholder('Selecione uma categoria')
            .addOptions(options),
    )
}

function helpContainer(
    client: CustomClient,
    ownerId: string,
    canSeeAdminCommands: boolean,
    content: string,
    selectedCategory?: string,
): ContainerComponentData {
    return {
        type: ComponentType.Container,
        accentColor: 0x8b5cf6,
        components: [
            {
                type: ComponentType.TextDisplay,
                content,
            },
            helpMenu(client, ownerId, canSeeAdminCommands, selectedCategory),
        ],
    }
}

export function helpHomePanel(
    client: CustomClient,
    ownerId: string,
    canSeeAdminCommands: boolean,
): ContainerComponentData {
    const grouped = categories(client, canSeeAdminCommands)
    const description = [
        '# Central de ajuda',
        '',
        'Use o menu abaixo para navegar pelos comandos.',
        '',
        ...[...grouped.entries()].map(([category, commands]) => `**${category}** — ${commands.length} comando(s)`),
    ]

    if (client.prefixCommandsEnabled) {
        description.push('', `Prefixo legado ativo: \`${client.prefix}\``)
    }

    return helpContainer(client, ownerId, canSeeAdminCommands, description.join('\n'))
}

export function helpCategoryPanel(
    client: CustomClient,
    ownerId: string,
    canSeeAdminCommands: boolean,
    category: string,
): ContainerComponentData {
    const commands = categories(client, canSeeAdminCommands).get(category)
    if (!commands) {
        return helpContainer(
            client,
            ownerId,
            canSeeAdminCommands,
            '# Categoria não encontrada\n\nSelecione uma categoria válida no menu abaixo.',
        )
    }

    const details = commands.map((name) => {
        const command = client.findCommand(name)
        return command ? `\`/${command.name}\` — ${command.description}` : `\`/${name}\``
    })

    return helpContainer(
        client,
        ownerId,
        canSeeAdminCommands,
        [`# Ajuda • ${category}`, '', ...details, '', '-# Use `/help comando` para detalhes de um comando.'].join('\n'),
        category,
    )
}

export function commandHelpPanel(
    client: CustomClient,
    ownerId: string,
    canSeeAdminCommands: boolean,
    query: string,
): ContainerComponentData {
    const command = client.findCommand(query.toLowerCase())
    if (!command || (command.adminOnly && !canSeeAdminCommands)) {
        return helpContainer(
            client,
            ownerId,
            canSeeAdminCommands,
            `# Comando não encontrado\n\nNenhum comando disponível foi encontrado para \`${query}\`.\nUse \`/help\` para abrir o painel por categoria.`,
        )
    }

    const aliases = command.aliases.length > 0
        ? command.aliases.map((alias) => `\`${alias}\``).join(', ')
        : '*Nenhum*'

    return helpContainer(
        client,
        ownerId,
        canSeeAdminCommands,
        [
            `# Detalhes do comando: \`/${command.name}\``,
            '',
            `**Descrição:** ${command.description}`,
            `**Categoria:** ${command.category}`,
            `**Como usar:** \`/${command.name}\``,
            `**Aliases legados:** ${aliases}`,
        ].join('\n'),
        command.category,
    )
}

export async function handleHelpCategoryInteraction(
    client: CustomClient,
    interaction: StringSelectMenuInteraction,
): Promise<boolean> {
    if (!interaction.customId.startsWith(`${HELP_CATEGORY_CUSTOM_ID}:`)) return false

    const ownerId = interaction.customId.slice(`${HELP_CATEGORY_CUSTOM_ID}:`.length)
    if (!ownerId || ownerId !== interaction.user.id) {
        await interaction.reply({
            content: 'Apenas o autor deste painel pode interagir com o menu de ajuda.',
            flags: MessageFlags.Ephemeral,
        })
        return true
    }

    const canSeeAdminCommands = interaction.memberPermissions?.has(PermissionFlagsBits.Administrator) ?? false
    const category = interaction.values[0]
    await interaction.update({
        flags: MessageFlags.IsComponentsV2,
        components: [helpCategoryPanel(client, ownerId, canSeeAdminCommands, category)],
    })
    return true
}
