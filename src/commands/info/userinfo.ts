import { ChatInputCommandInteraction, EmbedBuilder, SlashCommandBuilder } from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, { name: 'userinfo', description: 'Exibe informações de um usuário.', category: 'Informação', data: new SlashCommandBuilder().setName('userinfo').setDescription('Exibe informações de um usuário.').addUserOption((option) => option.setName('usuario').setDescription('Usuário consultado.').setRequired(false)) })
    }
    async executeMessage(): Promise<void> {}
    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        const user = interaction.options.getUser('usuario') ?? interaction.user
        const member = interaction.guild ? await interaction.guild.members.fetch(user.id).catch(() => null) : null
        const roles = member?.roles.cache.filter((role) => role.id !== interaction.guildId).map((role) => role.name).join(', ') || 'Nenhum'
        await interaction.reply({ embeds: [new EmbedBuilder().setTitle(user.tag).setThumbnail(user.displayAvatarURL()).addFields({ name: 'ID', value: user.id, inline: true }, { name: 'Conta criada', value: `<t:${Math.floor(user.createdTimestamp / 1000)}:R>`, inline: true }, { name: 'Cargos', value: roles }).setColor(0x5865f2)] })
    }
}
