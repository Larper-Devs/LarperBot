import {
    ActionRowBuilder,
    ButtonBuilder,
    ButtonInteraction,
    ButtonStyle,
    ComponentType,
    ContainerComponentData,
    Guild,
    MessageFlags,
    Role,
    StringSelectMenuBuilder,
    StringSelectMenuInteraction,
} from 'discord.js'

export const COLOR_ROLE_NAMES = [
    'Vermelho',
    'Branco',
    'Preto',
    'Vermelho Vinho',
    'Rosa',
    'Amarelo',
    'Verde',
    'Verde Escuro',
    'Azul',
    'Roxo',
    'Laranja',
    'Marrom',
] as const

type Technology = {
    name: string
    description: string
    style: ButtonStyle
}

const TECHNOLOGIES: Technology[] = [
    { name: 'JavaScript', description: 'Linguagem para web, automações e aplicações Node.js.', style: ButtonStyle.Primary },
    { name: 'TypeScript', description: 'JavaScript com tipagem estática e melhor suporte para projetos grandes.', style: ButtonStyle.Primary },
    { name: 'Python', description: 'Linguagem versátil para automação, dados, web e inteligência artificial.', style: ButtonStyle.Primary },
    { name: 'Java', description: 'Linguagem multiplataforma usada em backend, Android e sistemas corporativos.', style: ButtonStyle.Primary },
    { name: 'C#', description: 'Linguagem do ecossistema .NET, jogos com Unity e aplicações desktop.', style: ButtonStyle.Primary },
    { name: 'C++', description: 'Linguagem de alto desempenho usada em engines, sistemas e aplicações nativas.', style: ButtonStyle.Primary },
    { name: 'C', description: 'Linguagem de baixo nível usada em sistemas operacionais e embarcados.', style: ButtonStyle.Primary },
    { name: 'Go', description: 'Linguagem compilada focada em simplicidade, concorrência e serviços.', style: ButtonStyle.Primary },
    { name: 'Rust', description: 'Linguagem de alto desempenho com segurança de memória.', style: ButtonStyle.Primary },
    { name: 'PHP', description: 'Linguagem muito utilizada em aplicações web e CMS.', style: ButtonStyle.Primary },
    { name: 'Ruby', description: 'Linguagem expressiva conhecida pelo ecossistema Rails.', style: ButtonStyle.Primary },
    { name: 'Kotlin', description: 'Linguagem moderna para Android, backend e aplicações multiplataforma.', style: ButtonStyle.Primary },
    { name: 'Swift', description: 'Linguagem da Apple para iOS, macOS e demais plataformas do ecossistema.', style: ButtonStyle.Primary },
    { name: 'HTML', description: 'Linguagem de marcação usada para estruturar páginas web.', style: ButtonStyle.Secondary },
    { name: 'CSS', description: 'Linguagem usada para estilizar e organizar interfaces web.', style: ButtonStyle.Secondary },
    { name: 'React', description: 'Biblioteca para construção de interfaces com componentes.', style: ButtonStyle.Secondary },
    { name: 'Next.js', description: 'Framework React para aplicações web full-stack.', style: ButtonStyle.Secondary },
    { name: 'Node.js', description: 'Runtime JavaScript para servidores, ferramentas e bots.', style: ButtonStyle.Secondary },
    { name: 'discord.js', description: 'Biblioteca JavaScript para criar bots e integrações com Discord.', style: ButtonStyle.Secondary },
    { name: 'Vue', description: 'Framework progressivo para interfaces web.', style: ButtonStyle.Secondary },
    { name: 'Angular', description: 'Framework completo para aplicações web escaláveis.', style: ButtonStyle.Secondary },
    { name: 'Svelte', description: 'Framework que compila componentes para JavaScript otimizado.', style: ButtonStyle.Secondary },
    { name: 'SQL', description: 'Linguagem para consultar e manipular bancos de dados relacionais.', style: ButtonStyle.Secondary },
    { name: 'MongoDB', description: 'Banco de dados orientado a documentos, usado neste projeto.', style: ButtonStyle.Secondary },
    { name: 'Docker', description: 'Plataforma para empacotar e executar aplicações em containers.', style: ButtonStyle.Secondary },
]

function technologyKey(name: string): string {
    return name
        .toLowerCase()
        .replaceAll('#', '-sharp')
        .replaceAll('++', '-plus-plus')
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-|-$/g, '')
}

function panelContainer(content: string, components: ContainerComponentData['components']): ContainerComponentData {
    return {
        type: ComponentType.Container,
        accentColor: 0x8b5cf6,
        components: [
            { type: ComponentType.TextDisplay, content },
            ...components,
        ],
    }
}

function chunk<T>(items: T[], size: number): T[][] {
    const result: T[][] = []
    for (let index = 0; index < items.length; index += size) result.push(items.slice(index, index + size))
    return result
}

function roleByName(guild: Guild, name: string): Role | undefined {
    return guild.roles.cache.find((role) => role.name.toLowerCase() === name.toLowerCase())
}

export function findMemberRole(guild: Guild): Role | undefined {
    return roleByName(guild, 'Membro')
}

export function findColorRoles(guild: Guild): Role[] {
    return COLOR_ROLE_NAMES.flatMap((name) => {
        const role = roleByName(guild, name)
        return role ? [role] : []
    })
}

export function missingColorRoleNames(guild: Guild): string[] {
    return COLOR_ROLE_NAMES.filter((name) => !roleByName(guild, name))
}

export function verifyPanel(roleId: string): ContainerComponentData {
    return panelContainer(
        '# Verificação\n\nClique no botão abaixo para receber o cargo **Membro** e liberar o acesso ao servidor.',
        [new ActionRowBuilder<ButtonBuilder>().addComponents(
            new ButtonBuilder()
                .setCustomId(`panel:verify:${roleId}`)
                .setLabel('Receber cargo Membro')
                .setStyle(ButtonStyle.Success),
        )],
    )
}

export function colorsPanel(roles: Role[]): ContainerComponentData {
    return panelContainer(
        '# Cores\n\nEscolha uma cor para receber o cargo correspondente. Selecionar outra cor remove a anterior.',
        [new ActionRowBuilder<StringSelectMenuBuilder>().addComponents(
            new StringSelectMenuBuilder()
                .setCustomId('panel:colors')
                .setPlaceholder('Escolha uma cor')
                .setMinValues(0)
                .setMaxValues(1)
                .addOptions(roles.map((role) => ({ label: role.name, value: role.id }))),
        )],
    )
}

export function technologyPanel(): ContainerComponentData {
    const rows = chunk(TECHNOLOGIES, 5).map((items) => (
        new ActionRowBuilder<ButtonBuilder>().addComponents(items.map((technology) => (
            new ButtonBuilder()
                .setCustomId(`panel:tech:${technologyKey(technology.name)}`)
                .setLabel(technology.name)
                .setStyle(technology.style)
        )))
    ))

    return panelContainer(
        '# Tecnologias\n\nSelecione uma linguagem ou tecnologia para receber uma breve descrição.',
        rows,
    )
}

export async function handlePanelButton(interaction: ButtonInteraction): Promise<boolean> {
    if (interaction.customId.startsWith('panel:verify:')) {
        if (!interaction.guild) return true
        const roleId = interaction.customId.slice('panel:verify:'.length)
        const role = interaction.guild.roles.cache.get(roleId) ?? findMemberRole(interaction.guild)
        if (!role) {
            await interaction.reply({ content: 'O cargo `Membro` não foi encontrado.', flags: MessageFlags.Ephemeral })
            return true
        }

        const member = await interaction.guild.members.fetch(interaction.user.id)
        if (member.roles.cache.has(role.id)) {
            await interaction.reply({ content: 'Você já possui o cargo `Membro`.', flags: MessageFlags.Ephemeral })
            return true
        }

        await member.roles.add(role, 'Verificação pelo painel').catch(async () => {
            await interaction.reply({ content: 'Não consegui entregar o cargo. Verifique se o bot pode gerenciar esse cargo.', flags: MessageFlags.Ephemeral })
        })
        if (!interaction.replied) await interaction.reply({ content: 'Verificação concluída. O cargo `Membro` foi entregue.', flags: MessageFlags.Ephemeral })
        return true
    }

    if (!interaction.customId.startsWith('panel:tech:')) return false
    const key = interaction.customId.slice('panel:tech:'.length)
    const technology = TECHNOLOGIES.find((item) => technologyKey(item.name) === key)
    await interaction.reply({
        content: technology ? `**${technology.name}**\n${technology.description}` : 'Tecnologia não encontrada.',
        flags: MessageFlags.Ephemeral,
    })
    return true
}

export async function handlePanelColorSelection(interaction: StringSelectMenuInteraction): Promise<boolean> {
    if (interaction.customId !== 'panel:colors') return false
    if (!interaction.guild) return true

    const member = await interaction.guild.members.fetch(interaction.user.id)
    const roles = findColorRoles(interaction.guild)
    const selectedId = interaction.values[0]
    const toRemove = roles.filter((role) => role.id !== selectedId && member.roles.cache.has(role.id))

    if (toRemove.length > 0) await member.roles.remove(toRemove, 'Troca de cor pelo painel')
    if (selectedId) {
        const selected = roles.find((role) => role.id === selectedId)
        if (!selected) {
            await interaction.reply({ content: 'Essa cor não está mais disponível.', flags: MessageFlags.Ephemeral })
            return true
        }
        await member.roles.add(selected, 'Cor escolhida pelo painel')
        await interaction.reply({ content: `Sua cor foi atualizada para **${selected.name}**.`, flags: MessageFlags.Ephemeral })
    } else {
        await interaction.reply({ content: 'Suas cores foram removidas.', flags: MessageFlags.Ephemeral })
    }
    return true
}
