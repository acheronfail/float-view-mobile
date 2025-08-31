import z from 'zod';

/*
 * Commands (from the frontend)
 */

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
type empty = {};

type WsCommandDef = {
	scan: empty;
	connect: { address: string };
	disconnect: empty;
	data: { log: false } | { log: true; intervalMs: number };
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
	disconnect: () => ({ cmd: 'disconnect', data: {} }),
	data: (data) => ({ cmd: 'data', data })
};

export const sendCommand = <K extends keyof WsCommandDef>(
	ws: WebSocket,
	cmd: K,
	data: WsCommandDef[K]
) => {
	ws.send(JSON.stringify(commandFactories[cmd](data)));
};

/*
 * Messages (from the backend)
 */

const createMessage = <T extends string, E extends string, D extends z.ZodTypeAny>(
	topic: T,
	event: E,
	data: D
) =>
	z.object({
		topic: z.literal(topic),
		event: z.literal(event),
		data
	});

/*
 * Common
 */

const ErrorData = z.object({ message: z.string() });
const EmptyData = z.object({});

/*
 * Socket messages
 */

export const SocketMessage = z.discriminatedUnion('event', [
	createMessage('socket', 'error', ErrorData),
	createMessage('socket', 'welcome', EmptyData)
]);
export type ZSocketMessage = z.infer<typeof SocketMessage>;

/*
 * Scan messages
 */

export const ScanStatus = z.union([z.literal('start'), z.literal('complete')]);
export type ZScanStatus = z.infer<typeof ScanStatus>;

export const ScanDevice = z.object({ address: z.string(), name: z.string() });
export type ZScanDevice = z.infer<typeof ScanDevice>;

export const ScanMessage = z.discriminatedUnion('event', [
	createMessage('scan', 'error', ErrorData),
	createMessage('scan', 'device', ScanDevice),
	createMessage('scan', 'status', z.object({ status: ScanStatus }))
]);
export type ZScanMessage = z.infer<typeof ScanMessage>;

/*
 * Connect messages
 */

const ConnectStatus = z.union([
	z.literal('start'),
	z.literal('connecting'),
	z.literal('connected'),
	z.literal('ready')
]);

export const ConnectMessage = z.discriminatedUnion('event', [
	createMessage('connect', 'error', ErrorData),
	createMessage('connect', 'disconnect', z.object({ reason: z.string() })),
	createMessage('connect', 'status', z.object({ status: ConnectStatus }))
]);
export type ZConnectMessage = z.infer<typeof ConnectMessage>;

/*
 * Data messages
 */

export const DataMessage = z.discriminatedUnion('event', [
	createMessage('data', 'error', ErrorData),
	createMessage(
		'data',
		'firmware',
		z.object({
			major: z.number(),
			minor: z.number(),
			name: z.string(),
			uuid: z.string(),
			isPaired: z.boolean(),
			betaVersion: z.number(),
			hwType: z.number()
		})
	),
	createMessage(
		'data',
		'log',
		z.object({
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
		})
	)
]);
export type ZDataMessage = z.infer<typeof DataMessage>;

/*
 * All messages
 */

export const WsMessage = z.discriminatedUnion('topic', [
	SocketMessage,
	ScanMessage,
	ConnectMessage,
	DataMessage
]);

export type ZWsMessage = z.infer<typeof WsMessage>;

export const parseMessage = (ev: MessageEvent<unknown>): ZWsMessage => {
	if (typeof ev.data !== 'string') {
		throw new Error('Invalid message format');
	}

	const json = JSON.parse(ev.data);
	try {
		return WsMessage.parse(json);
	} catch (err) {
		throw { err, json };
	}
};
