import { Commands } from '../../structures/Commands'
import { CustomClient } from '../../structures/Client'
import { Message } from 'stoat.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {

            name: 'padd',
            description: 'Adiciona um item na database.',
            aliases: ['playlistadd'],
            category: 'Moderação',
            howToUse: 'padd [playlist] [url]'
        })
    }

    run = async (client: CustomClient, message: Message, args: string[]) => {

        return message.channel?.sendMessage("vai se foder")
    }
}