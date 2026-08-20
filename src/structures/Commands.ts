import { CustomClient } from './Client'

class Commands {
    
    client: CustomClient
    name: string
    description: string
    aliases: string[]
    category: string
    howToUse: string
    
    constructor(client: CustomClient, options: { name: string, description: string, aliases: string[], category: string, howToUse: string }) {
        this.client = client
        this.name = options.name
        this.description = options.description
        this.aliases = options.aliases
        this.category = options.category
        this.howToUse = options.howToUse
    }
}

export { Commands }