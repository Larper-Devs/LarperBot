import { Commands } from '../../structures/Commands'
import { CustomClient } from '../../structures/Client'
import { Message } from 'stoat.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'clear',
            description: 'Serve para apagar uma quantidade de mensagens em um chat.',
            aliases: ['mod', 'c'],
            category: 'Moderação',
            howToUse: 'clear [quantidade]'
        })
    }

    run = async (client: CustomClient, message: Message, args: string[]) => {
        if (!args[0]) {
            return message.channel?.sendMessage({
                embeds: [{
                    title: '❌ Uso incorreto',
                    description: 'Use: `clear [quantidade]`',
                    colour: '#ED4245'
                }]
            })
        }

        const amount = parseInt(args[0]);

        if (isNaN(amount) || amount < 1 || amount > 100) {
            return message.channel?.sendMessage({
                embeds: [{
                    title: '❌ Quantidade inválida',
                    description: 'Informe um número entre 1 e 100.',
                    colour: '#ED4245'
                }]
            })
        }

        try {
            const messages = await message.channel?.fetchMessages({
                limit: amount + 1,
                sort: 'Latest'
            });

            const ids = messages!.map((m) => m.id);
            await message.channel?.deleteMessages(ids);
        } catch (err) {
            console.error('Erro ao deletar mensagens:', err);
            await message.channel?.sendMessage({
                embeds: [{
                    title: '❌ Erro',
                    description: 'Não foi possível deletar as mensagens. Verifique se o bot tem permissão `ManageMessages`.',
                    colour: '#ED4245'
                }]
            })
        }
    }
}
