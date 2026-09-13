import { CustomClient } from './Client.js'
import { Logger } from './Logger.js'

class Event {

    client: CustomClient
    logger: Logger
    name: string

    constructor(client: CustomClient, options: { name: string }) {
        this.client = client
        this.logger = client.logger
        this.name = options.name
    }
}

export { Event }
