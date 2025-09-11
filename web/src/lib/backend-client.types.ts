import z from 'zod';
import { VescCommand, vescResponseSchemas } from './vesc.commands';
import { refloatResponseSchemas, RefloatRtData } from './refloat.commands';

/*
 * Commands (frontend -> backend)
 */

export const BackendCommand = z.discriminatedUnion('kind', [
	z.object({ kind: z.literal('scan') }),
	z.object({ kind: z.literal('connect'), address: z.string() }),
	z.object({ kind: z.literal('disconnect'), address: z.string() }),
	z.object({ kind: z.literal('logStart'), address: z.string(), intervalMs: z.number() }),
	z.object({ kind: z.literal('logStop'), address: z.string() }),
	z.object({ kind: z.literal('command'), address: z.string(), cmd: VescCommand })
]);
export type ZBackendCommand = z.infer<typeof BackendCommand>;

/*
 * Messages (backend -> frontend)
 */

const CommonError = { message: z.string() };
export const BackendMessage = z.discriminatedUnion('kind', [
	// socket
	z.object({ kind: z.literal('socketWelcome') }),
	z.object({ kind: z.literal('socketError') }).extend(CommonError),

	// scan
	z.object({ kind: z.literal('scanStart') }),
	z.object({ kind: z.literal('scanComplete') }),
	z.object({ kind: z.literal('scanDevice'), address: z.string(), name: z.string() }),
	z.object({ kind: z.literal('scanError') }).extend(CommonError),

	// connect
	z.object({ kind: z.literal('connectStart'), address: z.string() }),
	z.object({ kind: z.literal('connectComplete'), address: z.string() }),
	z.object({ kind: z.literal('connectReady'), address: z.string() }),
	z.object({ kind: z.literal('connectError') }).extend(CommonError),

	// disconnect
	z.object({ kind: z.literal('disconnectUnexpected'), address: z.string(), reason: z.string() }),
	z.object({ kind: z.literal('disconnectComplete'), address: z.string() }),
	z
		.object({ kind: z.literal('disconnectError'), address: z.string().optional() })
		.extend(CommonError),

	// commands
	z.object({ kind: z.literal('commandError') }).extend(CommonError),

	...vescResponseSchemas,
	...refloatResponseSchemas,

	// log
	z.object({ kind: z.literal('logStart'), address: z.string(), intervalMs: z.number() }),
	z.object({ kind: z.literal('logStop'), address: z.string() }),
	z.object({ kind: z.literal('logData'), address: z.string(), data: RefloatRtData }),
	z.object({ kind: z.literal('logError') }).extend(CommonError)
]);
export type ZBackendMessage = z.infer<typeof BackendMessage>;

export const parseMessage = (ev: MessageEvent<unknown>): ZBackendMessage => {
	if (typeof ev.data !== 'string') {
		throw new Error('Invalid message format');
	}

	const json = JSON.parse(ev.data);
	try {
		return BackendMessage.parse(json);
	} catch (err) {
		console.error({ err, json });
		throw err;
	}
};
