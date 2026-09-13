import {
    ChatInputCommandInteraction,
    EmbedBuilder,
    Message,
    MessageFlags,
    SlashCommandBuilder,
} from 'discord.js'
import { consumeResumeUse, parseResumeUrl, ResumeContentError, summarizeUrl } from '../../services/ResumeService.js'
import { Commands } from '../../structures/Commands.js'
import { CustomClient } from '../../structures/Client.js'

export default class extends Commands {
    constructor(client: CustomClient) {
        super(client, {
            name: 'resume',
            description: 'Resume o conteúdo de um site usando o Jina Reader.',
            category: 'Utilitários',
            data: new SlashCommandBuilder()
                .setName('resume')
                .setDescription('Resume o conteúdo de um site usando o Jina Reader.')
                .addStringOption((option) => option
                    .setName('url')
                    .setDescription('URL pública do site que será resumido.')
                    .setMaxLength(2_000)
                    .setRequired(true)),
        })
    }

    async executeMessage(_client: CustomClient, message: Message, _args: string[]): Promise<void> {
        await message.reply('Use o comando slash `/resume` e informe a URL do site.')
    }

    async executeInteraction(_client: CustomClient, interaction: ChatInputCommandInteraction): Promise<void> {
        let sourceUrl: string

        try {
            sourceUrl = parseResumeUrl(interaction.options.getString('url', true))
        } catch (error) {
            await interaction.reply({
                content: error instanceof Error ? error.message : 'Informe uma URL válida.',
                flags: MessageFlags.Ephemeral,
            })
            return
        }

        const rateLimit = consumeResumeUse(interaction.user.id)
        if (!rateLimit.allowed) {
            const seconds = Math.max(1, Math.ceil((rateLimit.retryAfterMs ?? 60_000) / 1_000))
            await interaction.reply({
                content: `Você atingiu o limite de 3 resumos por minuto. Tente novamente em **${seconds}s**.`,
                flags: MessageFlags.Ephemeral,
            })
            return
        }

        await interaction.deferReply()

        try {
            const result = await summarizeUrl(sourceUrl)
            await interaction.editReply({
                embeds: [new EmbedBuilder()
                    .setTitle(`Resumo: ${result.title}`)
                    .setURL(result.sourceUrl)
                    .setDescription(result.summary)
                    .addFields({ name: 'Fonte', value: `[Abrir site](${result.sourceUrl})` })
                    .setFooter({ text: 'Markdown extraído pelo Jina Reader • limite: 3 usos por minuto' })
                    .setColor(0x8b5cf6)],
                allowedMentions: { parse: [] },
            })
        } catch (error) {
            this.logger.error('Falha ao resumir URL', error)
            await interaction.editReply({
                content: error instanceof ResumeContentError
                    ? error.message
                    : 'Não foi possível ler esse site agora. Confirme se a URL é pública e tente novamente em alguns instantes.',
            })
        }
    }
}
