import z from 'zod';
import { VescCommand } from './vesc.commands';

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

export const VFirmware = z.object({
	major: z.number(),
	minor: z.number(),
	name: z.string(),
	uuid: z.string(),
	isPaired: z.boolean(),
	betaVersion: z.number(),
	hwType: z.number()
});
export type ZVFirmware = z.infer<typeof VFirmware>;

export const VGetValues = z.object({
	tempMosfet: z.number(),
	tempMotor: z.number(),
	currentMotor: z.number(),
	currentBattery: z.number(),
	id: z.number(),
	iq: z.number(),
	dutyCycle: z.number(),
	erpm: z.number(),
	voltage: z.number(),
	ampHoursConsumed: z.number(),
	ampHoursCharged: z.number(),
	wattHoursConsumed: z.number(),
	wattHoursCharged: z.number(),
	tachValue: z.number(),
	tachAbs: z.number(),
	faultCode: z.number()
});
export type ZVGetValues = z.infer<typeof VGetValues>;

export const RefloatRtData = z.object({
	balanceCurrent: z.number(),
	imuBalancePitch: z.number(),
	imuRoll: z.number(),
	satState: z.number(),
	state: z.number(),
	footpadState: z.number(),
	beepReason: z.number(),
	adc1: z.number(),
	adc2: z.number(),
	setpoint: z.number(),
	setpointAtr: z.number(),
	setpointBrakeTilt: z.number(),
	setpointTorqueTilt: z.number(),
	setpointTurnTilt: z.number(),
	setpointRemote: z.number(),
	imuPitch: z.number(),
	motorFilterCurrent: z.number(),
	atrAccelerationDiff: z.number(),
	boosterCurrent: z.number(),
	motorDirCurrent: z.number(),
	remoteInput: z.number()
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

	// commands
	z.object({ kind: z.literal('commandError') }).extend(CommonError),

	// vesc commands
	z.object({
		kind: z.literal('vescFirmware'),
		address: z.string(),
		firmware: VFirmware
	}),
	z.object({
		kind: z.literal('vescGetValues'),
		address: z.string(),
		getValues: VGetValues
	}),

	// refloat commands
	z.object({
		kind: z.literal('refloatRtData'),
		address: z.string(),
		refloatRtData: RefloatRtData
	}),

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
