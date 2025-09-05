import z from 'zod';
import { CommPacketId, VescCommand } from './vesc.commands';

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

export const RefloatRtData = z.object({
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
export type ZRefloatRtData = z.infer<typeof RefloatRtData>;

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
