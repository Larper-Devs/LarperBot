import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'avatar', description: 'Exibe o avatar de um usuário.', category: 'Informação', data: new SlashCommandBuilder().setName('avatar').setDescription('Exibe o avatar de um usuário.').addUserOption((option) => option.setName('usuario').setDescription('Usuário consultado.').setRequired(false)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        const user = interaction.options.getUser('usuario') ?? interaction.user
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle(`Avatar de ${user.tag}`).setImage(user.displayAvatarURL({ size: 1024 })).setColor(0x5865f2)] })
    }
}
