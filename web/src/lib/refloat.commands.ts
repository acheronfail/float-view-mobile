import z, { number } from 'zod';
import { VescDataType } from './vesc.types';

export const RefloatPackageId = 101;

// https://github.com/lukash/refloat/blob/5bde1f98a344410085960f88aa13d6c94f3dc24b/src/main.c#L1232
export enum RefloatCommPacketId {
	/** get version / package info */
	COMMAND_INFO = 0,
	/** get rt data */
	COMMAND_GET_RTDATA = 1,
	/** runtime tuning (don't write to eeprom) */
	COMMAND_RT_TUNE = 2,
	/** set tune to defaults (no eeprom) */
	COMMAND_TUNE_DEFAULTS = 3,
	/** save config to eeprom */
	COMMAND_CFG_SAVE = 4,
	/** restore config from eeprom */
	COMMAND_CFG_RESTORE = 5,
	/** make runtime changes to startup/etc */
	COMMAND_TUNE_OTHER = 6,
	/** move motor while board is idle */
	COMMAND_RC_MOVE = 7,
	/** change booster settings */
	COMMAND_BOOSTER = 8,
	/** print verbose info */
	COMMAND_PRINT_INFO = 9,
	/** send all data, compact */
	COMMAND_GET_ALLDATA = 10,
	/** generic cmd for sending data, used for testing/tuning new features */
	COMMAND_EXPERIMENT = 11,
	COMMAND_LOCK = 12,
	COMMAND_HANDTEST = 13,
	COMMAND_TUNE_TILT = 14,
	COMMAND_FLYWHEEL = 22,
	COMMAND_REALTIME_DATA = 31,
	COMMAND_REALTIME_DATA_IDS = 32,
	COMMAND_ALERTS_LIST = 35,
	COMMAND_ALERTS_CONTROL = 36,
	COMMAND_DATA_RECORD_REQUEST = 41

	// commands above 200 are unstable and can change protocol at any time
}

const id = (id: RefloatCommPacketId) => z.literal(id).describe('RefloatCommPacketId');

/**
 * The arguments to send with each command.
 * Note that responses are handled in the VescCommandLayer classes.
 * TODO: finish off the rest of these as we need them
 * https://github.com/lukash/refloat/blob/5bde1f98a344410085960f88aa13d6c94f3dc24b/src/main.c#L2133
 */
export const RefloatCommand = z.discriminatedUnion('kind', [
	z.object({
		kind: id(RefloatCommPacketId.COMMAND_INFO),
		version: number(VescDataType.Int8)
	}),
	z.object({
		kind: id(RefloatCommPacketId.COMMAND_GET_RTDATA)
	})
]);

/*
 * Responses
 */

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

export const refloatResponseSchemas = [
	z.object({
		kind: z.literal('refloatRtData'),
		address: z.string(),
		refloatRtData: RefloatRtData
	})
];
