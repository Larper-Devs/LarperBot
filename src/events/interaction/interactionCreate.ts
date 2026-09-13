import { Interaction, MessageFlags, PermissionFlagsBits } from 'discord.js'
import { Event } from '../../structures/Event.js'
import { CustomClient } from '../../structures/Client.js'
import { handleBlackjackInteraction } from '../../services/GameService.js'
import { handlePollInteraction } from '../../services/PollService.js'
import { handleColorSelection } from '../../services/WelcomeService.js'
import { handleHelpCategoryInteraction } from '../../services/HelpService.js'
import { handlePanelButton, handlePanelColorSelection } from '../../services/PanelService.js'

export default class extends Event {
    constructor(client: CustomClient) {
        super(client, { name: 'interactionCreate' })
    }

    run = async (interaction: Interaction): Promise<void> => {
        if (interaction.isAutocomplete()) {
            const command = this.client.findCommand(interaction.commandName)
            if (!command) return

            try {
                await command.executeAutocomplete(this.client, interaction)
            } catch (error) {
                this.logger.error(`Erro no autocomplete ${interaction.commandName}`, error)
                await interaction.respond([]).catch(() => undefined)
            }
            return
        }

        if (interaction.isButton()) {
            if (await handleBlackjackInteraction(interaction)) return
            if (await handlePollInteraction(interaction)) return
            if (await handlePanelButton(interaction)) return
            return
        }

        if (interaction.isStringSelectMenu()) {
            if (await handleColorSelection(interaction)) return
            if (await handlePanelColorSelection(interaction)) return
            if (await handleHelpCategoryInteraction(this.client, interaction)) return
            return
        }

        if (!interaction.isChatInputCommand()) return;

        const command = this.client.findCommand(interaction.commandName);
        if (!command) {
            await interaction.reply({ content: 'Esse comando não está disponível.', flags: MessageFlags.Ephemeral });
            return;
        }

        if (command.adminOnly && !interaction.memberPermissions?.has(PermissionFlagsBits.Administrator)) {
            await interaction.reply({
                content: 'Apenas administradores podem usar este comando.',
                flags: MessageFlags.Ephemeral,
            })
            return
        }

        try {
            await command.executeInteraction(this.client, interaction);
        } catch (error) {
            this.logger.error(`Erro no slash command ${interaction.commandName}`, error);

            if (interaction.replied || interaction.deferred) {
                await interaction.followUp({ content: 'Ocorreu um erro ao executar esse comando.', flags: MessageFlags.Ephemeral }).catch(() => undefined);
            } else {
                await interaction.reply({ content: 'Ocorreu um erro ao executar esse comando.', flags: MessageFlags.Ephemeral }).catch(() => undefined);
            }
        }
    }
}
