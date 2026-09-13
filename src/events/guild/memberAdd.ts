import { GuildMember } from 'discord.js'
import { Event } from '../../structures/Event.js'
import { CustomClient } from '../../structures/Client.js'
import { sendWelcome } from '../../services/WelcomeService.js'

export default class extends Event {
    constructor(client: CustomClient) {
        super(client, { name: 'guildMemberAdd' })
    }

    run = async (member: GuildMember): Promise<void> => {
        await sendWelcome(this.client, member)
    }
}
