import { Commands } from '../../structures/Commands'
import { CustomClient } from '../../structures/Client'
import { Message } from 'stoat.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'help',
            description: 'Exibe a lista de comandos disponíveis ou informações detalhadas sobre um comando específico.',
            aliases: ['ajuda', 'h'],
            category: 'Informação',
            howToUse: 'help [comando]'
        })
    }

    run = async (client: CustomClient, message: Message, args: string[]) => {
        const prefix = process.env.DEFAULT_PREFIX || '!'

        // Se o usuário solicitou detalhes de um comando específico (ex: !help padd)
        if (args.length > 0 && args[0].trim().length > 0) {
            const query = args[0].toLowerCase()
            const cmd = client.commands.find(
                (c) => c.name.toLowerCase() === query || (c.aliases && c.aliases.map(a => a.toLowerCase()).includes(query))
            )

            if (!cmd) {
                return message.channel?.sendMessage({
                    embeds: [
                        {
                            title: '❌ Comando não encontrado',
                            description: `Nenhum comando foi encontrado para \`${query}\`.\nUse \`${prefix}help\` para listar todos os comandos disponíveis.`,
                            colour: '#ED4245'
                        }
                    ]
                })
            }

            const aliasesText = cmd.aliases && cmd.aliases.length > 0
                ? cmd.aliases.map(a => `\`${a}\``).join(', ')
                : '*Nenhum*'

            return message.channel?.sendMessage({
                embeds: [
                    {
                        title: `📖 Detalhes do Comando: ${prefix}${cmd.name}`,
                        description: [
                            `**Descrição:** ${cmd.description || 'Sem descrição informada.'}`,
                            `**Categoria:** ${cmd.category || 'Geral'}`,
                            `**Como Usar:** \`${prefix}${cmd.howToUse || cmd.name}\``,
                            `**Aliases:** ${aliasesText}`
                        ].join('\n\n'),
                        colour: '#5865F2'
                    }
                ]
            })
        }

        // Listagem geral de todos os comandos agrupados por categoria
        const categoriesMap = new Map<string, typeof client.commands>()

        for (const cmd of client.commands) {
            const categoryName = cmd.category || 'Outros'
            if (!categoriesMap.has(categoryName)) {
                categoriesMap.set(categoryName, [])
            }
            categoriesMap.get(categoryName)!.push(cmd)
        }

        const lines: string[] = [
            `Use \`${prefix}help [comando]\` para ver mais informações sobre um comando.\n`
        ]

        for (const [category, cmds] of categoriesMap.entries()) {
            const cmdList = cmds.map(c => `\`${prefix}${c.name}\``).join(', ')
            lines.push(`### 📁 ${category}`)
            lines.push(`${cmdList}\n`)
        }

        return message.channel?.sendMessage({
            embeds: [
                {
                    title: '📋 Lista de Comandos',
                    description: lines.join('\n'),
                    colour: '#5865F2'
                }
            ]
        })
    }
}
