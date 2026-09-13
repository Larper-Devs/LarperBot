import { config } from 'dotenv'
import { REST, Routes } from 'discord.js'
import { CustomClient } from './structures/Client.js'

config({ path: '.env' })

const token = process.env.DISCORD_TOKEN ?? process.env.TOKEN
const clientId = process.env.CLIENT_ID
const guildId = process.env.GUILD_ID

if (!token || !clientId) {
    throw new Error('Defina DISCORD_TOKEN/TOKEN e CLIENT_ID no arquivo .env.')
}

const client = new CustomClient()
await client.loadCommands()

const rest = new REST({ version: '10' }).setToken(token)
const route = guildId
    ? Routes.applicationGuildCommands(clientId, guildId)
    : Routes.applicationCommands(clientId)

await rest.put(route, {
    body: client.commandList.map((command) => command.data.toJSON()),
})

console.log(`${client.commandList.length} slash command(s) registrado(s)${guildId ? ' no servidor de desenvolvimento' : ' globalmente'}.`)
