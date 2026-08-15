import { client } from "../src/lib.ts";

const held = await client.connect();

if (held.inspect === undefined) {
	throw new Error("the probe was granted no inspect seat");
}

await held.inspect.serve({
	"server.status": () => ({ port: held.control.port() ?? null }),
});

setInterval(() => undefined, 1000);
