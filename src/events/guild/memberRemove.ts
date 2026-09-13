import { GuildMember } from 'discord.js'
import { Event } from '../../structures/Event.js'
import { CustomClient } from '../../structures/Client.js'
import { sendLeave } from '../../services/WelcomeService.js'

export default class extends Event {
    constructor(client: CustomClient) {
        super(client, { name: 'guildMemberRemove' })
    }

    run = async (member: GuildMember): Promise<void> => {
        await sendLeave(this.client, member)
    }
}
