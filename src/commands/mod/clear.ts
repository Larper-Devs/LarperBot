import {
    ChatInputCommandInteraction,
    EmbedBuilder,
    Message,
    MessageFlags,
    PermissionFlagsBits,
    SlashCommandBuilder,
} from 'discord.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'clear',
            description: 'Apaga mensagens recentes de um canal.',
            aliases: ['mod', 'c'],
            category: 'Moderação',
            howToUse: 'clear [quantidade]',
            data: new SlashCommandBuilder()
                .setName('clear')
                .setDescription('Apaga mensagens recentes do canal.')
                .addIntegerOption((option) => option
                    .setName('quantidade')
                    .setDescription('Quantidade de mensagens, entre 1 e 100.')
                    .setMinValue(1)
                    .setMaxValue(100)
                    .setRequired(true)),
        })
    }

    async executeMessage(_client: CustomClient, message: Message, args: string[]): Promise<void> {
        if (!message.member?.permissions.has(PermissionFlagsBits.ManageMessages)) {
            await message.reply({ embeds: [this.errorEmbed('Você precisa da permissão `Gerenciar Mensagens`.')] })
            return
        }

        const amount = Number.parseInt(args[0] ?? '', 10)
        if (!Number.isInteger(amount) || amount < 1 || amount > 100) {
            await message.reply({ embeds: [this.errorEmbed('Use uma quantidade entre 1 e 100.')] })
            return
        }

        if (!message.channel.isTextBased() || !('bulkDelete' in message.channel)) {
            await message.reply({ embeds: [this.errorEmbed('Este comando só funciona em canais de texto.')] })
            return
        }

        try {
            const deleted = await message.channel.bulkDelete(amount, true)
            const confirmation = await message.channel.send(`🧹 ${deleted.size} mensagem(ns) apagada(s).`)
            setTimeout(() => confirmation.delete().catch(() => undefined), 3_000)
        } catch (error) {
            this.logger.error('Erro ao apagar mensagens', error)
            await message.reply({ embeds: [this.errorEmbed('Não foi possível apagar as mensagens. Verifique a permissão e se elas têm menos de 14 dias.')] })
        }
    }

    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        if (!interaction.memberPermissions?.has(PermissionFlagsBits.ManageMessages)) {
            await interaction.reply({ embeds: [this.errorEmbed('Você precisa da permissão `Gerenciar Mensagens`.')], flags: MessageFlags.Ephemeral })
            return
        }

        const amount = interaction.options.getInteger('quantidade', true)
        if (!interaction.channel || !('bulkDelete' in interaction.channel)) {
            await interaction.reply({ embeds: [this.errorEmbed('Este comando só funciona em canais de texto.')], flags: MessageFlags.Ephemeral })
            return
        }

        try {
            const deleted = await interaction.channel.bulkDelete(amount, true)
            await interaction.reply({ content: `🧹 ${deleted.size} mensagem(ns) apagada(s).`, flags: MessageFlags.Ephemeral })
        } catch (error) {
            this.logger.error('Erro ao apagar mensagens via slash command', error)
            await interaction.reply({ embeds: [this.errorEmbed('Não foi possível apagar as mensagens.')], flags: MessageFlags.Ephemeral })
        }
    }

    private errorEmbed(description: string): EmbedBuilder {
        return new EmbedBuilder()
            .setTitle('Erro')
            .setDescription(description)
            .setColor(0xed4245)
    }
}
