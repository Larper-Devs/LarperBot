import { config } from "dotenv";
import { CustomClient } from "./src/structures/Client";
import { writeOnLog } from "./src/structures/Logger";
config({ path: ".env" });

const originalDebug = console.debug;
console.debug = (...args: any[]) => {
  if (typeof args[0] === "string" && args[0].includes("during hydration")) {
    return;
  }
  originalDebug(...args);
};

new CustomClient(`${process.env.TOKEN}`);
process.on(
  "unhandledRejection",
  (
    err: { code: string; message: string },
    reason: { stack: string | undefined },
  ) => {
    writeOnLog(`${err.message}-${err.code} Location: ${reason.stack}`);
  },
);
