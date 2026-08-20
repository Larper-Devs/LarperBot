import { CustomClient } from './Client'

class Event {

    client: CustomClient
    name: string

    constructor(client: CustomClient, options: { name: string }) {
        this.client = client
        this.name = options.name
    }
}

export { Event }