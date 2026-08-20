import { config } from 'dotenv'
import { CustomClient } from './src/structures/Client'
import { writeOnLog } from './src/functions/log'
config({ path: '.env' })

const client = new CustomClient()

client.loginBot(`${process.env.TOKEN}`)

process.on('unhandledRejection', (err: { code: string, message: string }, reason: { stack: string | undefined }) => {
    writeOnLog(`${err.message}-${err.code} Location: ${reason.stack}`)
})