import { CustomClient } from './Client.js'
import { Logger } from './Logger.js'
import {
    AutocompleteInteraction,
    ChatInputCommandInteraction,
    Message,
    PermissionFlagsBits,
    SlashCommandBuilder,
} from 'discord.js'

type CommandOptions = {
    name: string
    description: string
    aliases?: string[]
    category?: string
    howToUse?: string
    adminOnly?: boolean
    data?: CommandData
}

type CommandData = {
    toJSON: SlashCommandBuilder['toJSON']
    setDefaultMemberPermissions: (...args: Parameters<SlashCommandBuilder['setDefaultMemberPermissions']>) => unknown
}

const ADMIN_ONLY_COMMANDS = new Set([
    'automod',
    'ban',
    'clear',
    'cores',
    'kick',
    'level-config',
    'lock',
    'novidades',
    'timeout',
    'unban',
    'unlock',
    'untimeout',
    'warn',
    'warnings',
    'welcome',
    'regras',
    'tech',
    'verify',
])

abstract class Commands {
    
    client: CustomClient
    logger: Logger
    name: string
    description: string
    aliases: string[]
    category: string
    howToUse: string
    adminOnly: boolean
    data: CommandData
    
    constructor(client: CustomClient, options: CommandOptions) {
        this.client = client
        this.logger = client.logger
        this.name = options.name
        this.description = options.description
        this.aliases = options.aliases ?? []
        this.category = options.category ?? 'Geral'
        this.howToUse = options.howToUse ?? options.name
        this.adminOnly = options.adminOnly ?? ADMIN_ONLY_COMMANDS.has(options.name)
        this.data = options.data ?? new SlashCommandBuilder()
            .setName(options.name)
            .setDescription(options.description)

        if (this.adminOnly) {
            this.data.setDefaultMemberPermissions(PermissionFlagsBits.Administrator)
        }
    }

    abstract executeMessage(client: CustomClient, message: Message, args: string[]): Promise<void>

    abstract executeInteraction(
        client: CustomClient,
        interaction: ChatInputCommandInteraction,
    ): Promise<void>

    async executeAutocomplete(
        _client: CustomClient,
        _interaction: AutocompleteInteraction,
    ): Promise<void> {}
}

export { Commands }
