import { config } from "dotenv";
import { CustomClient } from "./src/structures/Client.js";

config({ path: ".env" });

const client = new CustomClient();

process.on("unhandledRejection", (reason) => {
  client.logger.error("Promise rejeitada sem tratamento", reason);
});

process.on("uncaughtException", (error) => {
  client.logger.fatal("Exceção não capturada", error);
});

void client.start().catch((error) => {
  client.logger.fatal("Não foi possível iniciar o bot", error);
});
