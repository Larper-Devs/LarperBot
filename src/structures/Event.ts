import { CustomClient } from './Client'
import { Logger } from './Logger'

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