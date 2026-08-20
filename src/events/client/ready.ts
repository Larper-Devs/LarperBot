import { Event } from '../../structures/Event'
import { CustomClient } from '../../structures/Client'

export default class extends Event {
    constructor(client: CustomClient) {
        super(client, {
            name: 'ready'
        })
    }

    run = async () => {
        this.logger.success(`Bot ${this.client.user?.username} logado com sucesso em ${this.client.servers.size()} servidores.`);
    }
}