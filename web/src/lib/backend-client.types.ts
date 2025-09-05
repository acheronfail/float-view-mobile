import z from 'zod';
import { CommPacketId } from './vesc.commands';

/*
 * Commands (frontend -> backend)
 */

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
type empty = {};

// TODO: zod-ify so we can generate code from it as well
type WsCommandDef = {
	scan: empty;
	connect: { address: string };
	disconnect: { address: string };
	// TODO: JS doesn't need to know details of packets, since we'll need all that in the backends for bg processing
	// >>>>: so just send "high level" commands here and have the backends translate these into actual commands
	// TODO: look into ways of generating the Kotlin/Swift code necessary for this... precomputed payloads?
	// TODO: test getting float package type
	// https://github.com/lukash/refloat/blob/5bde1f98a344410085960f88aa13d6c94f3dc24b/src/main.c#L2142
	// PacketTypes.COMM_CUSTOM_APP_DATA, AppPacketTypes.FLOAT (101), RefloatCommPacketId.COMMAND_GET_RTDATA,
	command: { address: string; command: CommPacketId; payload: number[] };
	data: { address: string; log: false } | { address: string; log: true; intervalMs: number };
};

export type WsCommand<K extends keyof WsCommandDef = keyof WsCommandDef> = {
	cmd: K;
	data: WsCommandDef[K];
};

const commandFactories: {
	[K in keyof WsCommandDef]: (data: WsCommandDef[K]) => { cmd: K; data: WsCommandDef[K] };
} = {
	scan: () => ({ cmd: 'scan', data: {} }),
	connect: (data) => ({ cmd: 'connect', data }),
	command: (data) => ({ cmd: 'command', data }),
	disconnect: (data) => ({ cmd: 'disconnect', data }),
	data: (data) => ({ cmd: 'data', data })
};

export const sendCommand = <K extends keyof WsCommandDef>(
	ws: WebSocket,
	cmd: K,
	data: WsCommandDef[K]
) => {
	console.debug(['>', cmd].join(':'), data);
	ws.send(JSON.stringify(commandFactories[cmd](data)));
};

/*
 * Messages (backend -> frontend)
 */

export const CommandFirmware = z.object({
	major: z.number(),
	minor: z.number(),
	name: z.string(),
	uuid: z.string(),
	isPaired: z.boolean(),
	betaVersion: z.number(),
	hwType: z.number()
});
export type ZCommandFirmware = z.infer<typeof CommandFirmware>;

export const DataLogResponse = z.object({
	adc1: z.number(),
	adc2: z.number(),
	ah: z.number(),
	ahCharged: z.number().optional(),
	altitude: z.number(),
	bmsFault: z.number().optional(),
	bmsTemp: z.number().optional(),
	bmsTempBattery: z.number().optional(),
	currentBattery: z.number(),
	currentBooster: z.number().optional(),
	currentFieldWeakening: z.number().optional(),
	currentMotor: z.number(),
	distance: z.number(),
	duty: z.number(),
	erpm: z.number().optional(),
	gpsAccuracy: z.number(),
	gpsLatitude: z.number(),
	gpsLongitude: z.number(),
	motorFault: z.number(),
	pitch: z.number(),
	requestedAmps: z.number().optional(),
	roll: z.number(),
	setpoint: z.number().optional(),
	setpointAtr: z.number().optional(),
	setpointBreakTilt: z.number().optional(),
	setpointCarve: z.number().optional(),
	setpointRemote: z.number().optional(),
	setpointTorqueTilt: z.number().optional(),
	speed: z.number(),
	state: z.string(),
	stateRaw: z.number(),
	tempBattery: z.number().optional(),
	tempMosfet: z.number(),
	tempMotor: z.number(),
	time: z.number(),
	truePitch: z.number(),
	voltage: z.number(),
	wh: z.number(),
	whCharged: z.number().optional()
});
export type ZDataLogResponse = z.infer<typeof DataLogResponse>;

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
	z.object({ kind: z.literal('disconnectError'), address: z.string().optional() }).extend(CommonError),

	// command
	z.object({
		kind: z.literal('commandFirmware'),
		address: z.string(),
		firmware: CommandFirmware
	}),
	z
		.object({ kind: z.literal('commandError'), command: z.enum(CommPacketId).optional() })
		.extend(CommonError),

	// log
	z.object({ kind: z.literal('logStart'), address: z.string(), intervalMs: z.number() }),
	z.object({ kind: z.literal('logStop'), address: z.string() }),
	z.object({ kind: z.literal('logData'), address: z.string(), data: DataLogResponse }),
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
