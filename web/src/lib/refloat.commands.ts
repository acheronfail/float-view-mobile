import z from 'zod';
import { typedNumber, VescDataType } from './vesc.types';

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
		version: typedNumber(VescDataType.Int8)
	}),
	z.object({ kind: id(RefloatCommPacketId.COMMAND_GET_RTDATA) }),
	z.object({
		kind: id(RefloatCommPacketId.COMMAND_GET_ALLDATA),
		/**
		 * 1 = basic
		 * 2 = basic+motor
		 * 3 = basic+motor+battery
		 * 4 = basic+motor+battery+charge
		 */
		mode: typedNumber(VescDataType.Int8)
	})
]);

/*
 * Responses
 */

export const RefloatRtData = z.object({
	balanceCurrent: typedNumber(VescDataType.Float32),
	imuBalancePitch: typedNumber(VescDataType.Float32),
	imuRoll: typedNumber(VescDataType.Float32),
	satState: typedNumber(VescDataType.UInt8),
	state: typedNumber(VescDataType.UInt8),
	footpadState: typedNumber(VescDataType.UInt8),
	beepReason: typedNumber(VescDataType.UInt8),
	adc1: typedNumber(VescDataType.Float32),
	adc2: typedNumber(VescDataType.Float32),
	setpoint: typedNumber(VescDataType.Float32),
	setpointAtr: typedNumber(VescDataType.Float32),
	setpointBrakeTilt: typedNumber(VescDataType.Float32),
	setpointTorqueTilt: typedNumber(VescDataType.Float32),
	setpointTurnTilt: typedNumber(VescDataType.Float32),
	setpointRemote: typedNumber(VescDataType.Float32),
	imuPitch: typedNumber(VescDataType.Float32),
	motorFilterCurrent: typedNumber(VescDataType.Float32),
	atrAccelerationDiff: typedNumber(VescDataType.Float32),
	boosterCurrent: typedNumber(VescDataType.Float32),
	motorDirCurrent: typedNumber(VescDataType.Float32),
	remoteInput: typedNumber(VescDataType.Float32)
});
export type ZRefloatRtData = z.infer<typeof RefloatRtData>;

const RefloatAllDataFault = z.object({
	mode: z.literal('fault'),
	fault: typedNumber(VescDataType.UInt8)
});
export type ZRefloatAllDataFault = z.infer<typeof RefloatAllDataFault>;

const RefloatAllData1 = z.object({
	mode: z.literal('fault'),

	/** 10 */
	balanceCurrent: typedNumber(VescDataType.Double16),
	/** 10 */
	imuBalancePitch: typedNumber(VescDataType.Double16),
	/** 10 */
	imuRoll: typedNumber(VescDataType.Double16),

	satState: typedNumber(VescDataType.UInt8),
	state: typedNumber(VescDataType.UInt8),
	footpadState: typedNumber(VescDataType.UInt8),
	beepReason: typedNumber(VescDataType.UInt8),

	/** x / 50 */
	adc1: typedNumber(VescDataType.UInt8),
	/** x / 50 */
	adc2: typedNumber(VescDataType.UInt8),

	/** (x - 128) / 5 */
	setpoint: typedNumber(VescDataType.UInt8),
	/** (x - 128) / 5 */
	setpointAtr: typedNumber(VescDataType.UInt8),
	/** (x - 128) / 5 */
	setpointBrakeTilt: typedNumber(VescDataType.UInt8),
	/** (x - 128) / 5 */
	setpointTorqueTilt: typedNumber(VescDataType.UInt8),
	/** (x - 128) / 5 */
	setpointTurnTilt: typedNumber(VescDataType.UInt8),
	/** (x - 128) / 5 */
	setpointRemote: typedNumber(VescDataType.UInt8),

	/** 10 */
	imuPitch: typedNumber(VescDataType.Double16),
	/** x - 128 */
	boosterCurrent: typedNumber(VescDataType.UInt8),
	/** 10 */
	batteryVoltage: typedNumber(VescDataType.Double16),
	motorErpm: typedNumber(VescDataType.Int16),
	/** 10 */
	motorSpeed: typedNumber(VescDataType.Double16),
	/** 10 */
	motorCurrent: typedNumber(VescDataType.Double16),
	/** 10 */
	motorBatteryCurrent: typedNumber(VescDataType.Double16),
	/** (x - 128) / 100 */
	motorDutyRaw: typedNumber(VescDataType.UInt8),
	/** (x / 3) NOTE: magic value is 222 */
	focId: typedNumber(VescDataType.UInt8).optional()
});
export type ZRefloatAllData1 = z.infer<typeof RefloatAllData1>;

const RefloatAllData2 = RefloatAllData1.extend({
	distance: typedNumber(VescDataType.Float32),
	/** x / 2 */
	mosfetTemp: typedNumber(VescDataType.UInt8),
	/** x / 2 */
	motorTemp: typedNumber(VescDataType.UInt8),
	/** NOTE unused */
	batteryTemp: typedNumber(VescDataType.UInt8)
});
export type ZRefloatAllData2 = z.infer<typeof RefloatAllData2>;

const RefloatAllData3 = RefloatAllData2.extend({
	odometer: typedNumber(VescDataType.UInt32),
	/** 10 */
	ampHours: typedNumber(VescDataType.Double16),
	/** 10 */
	ampHoursCharged: typedNumber(VescDataType.Double16),
	/** 1 */
	wattHours: typedNumber(VescDataType.Double16),
	/** 1 */
	wattHoursCharged: typedNumber(VescDataType.Double16),
	/** x / 2 */
	batteryLevel: typedNumber(VescDataType.UInt8)
});
export type ZRefloatAllData3 = z.infer<typeof RefloatAllData3>;

const RefloatAllData4 = RefloatAllData3.extend({
	/** 10 */
	chargingCurrent: typedNumber(VescDataType.Double16),
	/** 10 */
	chargingVoltage: typedNumber(VescDataType.Double16)
});
export type ZRefloatAllData4 = z.infer<typeof RefloatAllData4>;

export const refloatResponseSchemas = [
	z.object({
		kind: z.literal('refloatRtData'),
		address: z.string(),
		refloatRtData: RefloatRtData
	}),
	z.object({
		kind: z.literal('refloatAllDataFault'),
		address: z.string(),
		refloatAllDataFault: RefloatAllDataFault
	}),
	z.object({
		kind: z.literal('refloatAllData1'),
		address: z.string(),
		refloatAllData1: RefloatAllData1
	}),
	z.object({
		kind: z.literal('refloatAllData2'),
		address: z.string(),
		refloatAllData2: RefloatAllData2
	}),
	z.object({
		kind: z.literal('refloatAllData3'),
		address: z.string(),
		refloatAllData3: RefloatAllData3
	}),
	z.object({
		kind: z.literal('refloatAllData4'),
		address: z.string(),
		refloatAllData4: RefloatAllData4
	})
];
