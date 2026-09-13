import {
  ChatInputCommandInteraction,
  MessageFlags,
  SlashCommandBuilder,
} from "discord.js";
import { Commands } from "../../structures/Commands.js";
import { CustomClient } from "../../structures/Client.js";
import { requireDatabase } from "../../utils/database.js";
import { getProfile } from "../../utils/profiles.js";

export default class extends Commands {
  constructor(client: CustomClient) {
    super(client, {
      name: "afk",
      description: "Define ou remove seu status de ausência.",
      category: "Convivência",
      data: new SlashCommandBuilder()
        .setName("afk")
        .setDescription("Define ou remove seu status de ausência.")
        .addStringOption((option) =>
          option
            .setName("mensagem")
            .setDescription("Mensagem de ausência; deixe vazio para remover.")
            .setMaxLength(200)
            .setRequired(false),
        ),
    });
  }
  async executeMessage(): Promise<void> {}
  async executeInteraction(
    client: CustomClient,
    interaction: ChatInputCommandInteraction,
  ): Promise<void> {
    if (!interaction.guild || !(await requireDatabase(client, interaction)))
      return;
    const profile = await getProfile(
      interaction.guild.id,
      interaction.user.id,
      interaction.user.username,
    );
    profile.afkMessage = interaction.options.getString("mensagem");
    await profile.save();
    await interaction.reply({
      content: profile.afkMessage
        ? `AFK ativado: ${profile.afkMessage}`
        : "Seu AFK foi removido.",
      flags: MessageFlags.Ephemeral,
    });
  }
}
