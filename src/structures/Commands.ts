import { CustomClient } from './Client'
import { Logger } from './Logger'

class Commands {
    
    client: CustomClient
    logger: Logger
    name: string
    description: string
    aliases: string[]
    category: string
    howToUse: string
    
    constructor(client: CustomClient, options: { name: string, description: string, aliases: string[], category: string, howToUse: string }) {
        this.client = client
        this.logger = client.logger
        this.name = options.name
        this.description = options.description
        this.aliases = options.aliases
        this.category = options.category
        this.howToUse = options.howToUse
    }
}

export { Commands }