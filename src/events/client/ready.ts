import { Event } from '../../structures/Event.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Event {
    constructor(client: CustomClient) {
        super(client, {
            name: 'clientReady'
        })
    }

    run = async () => {
        this.logger.success(
            `Bot ${this.client.user?.tag ?? 'desconhecido'} online em ${this.client.guilds.cache.size} servidor(es). Slash commands são o modo principal.`,
        );
    }
}
