import { randomInt } from 'crypto'
import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

const answers = ['Sim.', 'Não.', 'Provavelmente.', 'Melhor não contar com isso.', 'Com certeza!']
const actions: Record<string, string[]> = {
    hug: ['deu um abraço em'],
    kiss: ['deu um beijo em'],
    pat: ['fez carinho em'],
    cuddle: ['se aconchegou com'],
}

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'fun',
            description: 'Comandos sociais e de diversão.',
            category: 'Convivência',
            data: new SlashCommandBuilder()
                .setName('fun')
                .setDescription('Comandos sociais e de diversão.')
                .addSubcommand((subcommand) => subcommand
                    .setName('8ball')
                    .setDescription('Responde uma pergunta.')
                    .addStringOption((option) => option.setName('pergunta').setDescription('Pergunta feita.').setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('ship')
                    .setDescription('Calcula a compatibilidade entre duas pessoas.')
                    .addUserOption((option) => option.setName('usuario').setDescription('Pessoa comparada.').setRequired(true)))
                .addSubcommand((subcommand) => subcommand
                    .setName('social')
                    .setDescription('Interage com alguém.')
                    .addStringOption((option) => option
                        .setName('acao')
                        .setDescription('Ação.')
                        .addChoices(...Object.keys(actions).map((value) => ({ name: value, value })))
                        .setRequired(true))
                    .addUserOption((option) => option.setName('usuario').setDescription('Pessoa da interação.').setRequired(true))),
        })
    }

    async executeMessage(): Promise<void> {}

    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        const subcommand = interaction.options.getSubcommand()
        if (subcommand === '8ball') {
            await interaction.reply({ embeds: [new EmbedBuilder().setTitle('8ball').setDescription(answers[randomInt(0, answers.length)]).setColor(0x9b59b6)] })
            return
        }
        if (subcommand === 'ship') {
            const user = interaction.options.getUser('usuario', true)
            const score = randomInt(0, 101)
            await interaction.reply({ embeds: [new EmbedBuilder().setTitle('Compatibilidade').setDescription(`${interaction.user} + ${user}\n💞 **${score}%**`).setColor(0xe91e63)] })
            return
        }
        const action = interaction.options.getString('acao', true)
        const user = interaction.options.getUser('usuario', true)
        await interaction.reply(`${interaction.user} ${actions[action][0]} ${user}.`)
    }
}
