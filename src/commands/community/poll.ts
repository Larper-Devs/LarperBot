import { ChatInputCommandInteraction, SlashCommandBuilder } from 'discord.js'
import { createPoll } from '../../services/PollService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'poll',
            description: 'Cria uma enquete interativa.',
            category: 'Convivência',
            data: new SlashCommandBuilder()
                .setName('poll')
                .setDescription('Cria uma enquete interativa.')
                .addStringOption((option) => option.setName('pergunta').setDescription('Pergunta da enquete.').setMaxLength(200).setRequired(true))
                .addStringOption((option) => option.setName('opcao1').setDescription('Primeira opção.').setMaxLength(80).setRequired(true))
                .addStringOption((option) => option.setName('opcao2').setDescription('Segunda opção.').setMaxLength(80).setRequired(true))
                .addStringOption((option) => option.setName('opcao3').setDescription('Terceira opção.').setMaxLength(80).setRequired(false)),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        const options = [interaction.options.getString('opcao1', true), interaction.options.getString('opcao2', true)]
        const option3 = interaction.options.getString('opcao3')
        if (option3) options.push(option3)
        await createPoll(interaction, interaction.options.getString('pergunta', true), options)
    }
}
