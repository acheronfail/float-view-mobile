import type { FirmwareVersion } from '../../../fixture/firmware.ts';
import type { RefloatVersion } from '../../../fixture/refloat.ts';
import type { TelemetrySample } from '../../../fixture/telemetry.ts';
import type { FixtureTopology } from '../../../fixture/topology.ts';

export const FIXTURE_DEV_ORIGIN = 'http://127.0.0.1:11004';
export const FIXTURE_DEV_PORT = 11004;

export interface FixtureConfiguration {
	topology: FixtureTopology;
	canId: number;
	firmware: FirmwareVersion;
	refloat: RefloatVersion;
}

export interface SerialPortOption {
	path: string;
	label: string;
}

export interface FixtureCounters {
	writes: number;
	responses: number;
	telemetry: number;
	dropped: number;
}

export interface FixtureSnapshot {
	revision: number;
	serial: {
		status: 'detached' | 'attaching' | 'attached';
		path?: string;
		error?: string;
	};
	configuration: FixtureConfiguration;
	power: 'off' | 'on';
	ble: 'unavailable' | 'idle' | 'advertising' | 'connected';
	ride: 'idle' | 'riding';
	telemetry?: TelemetrySample;
	counters: FixtureCounters;
	diagnostics: readonly string[];
}

export type FixtureAction =
	| { kind: 'attach'; path: string }
	| { kind: 'detach' }
	| { kind: 'configure'; configuration: FixtureConfiguration }
	| { kind: 'power-on' }
	| { kind: 'power-off' }
	| { kind: 'start-riding' }
	| { kind: 'stop-riding' };

export const firmwareOptions: readonly FirmwareVersion[] = ['6.02', '6.05', '6.06'];
export const refloatOptions: readonly RefloatVersion[] = ['1.0.0', '1.1.0', '1.2.3'];
export const topologyOptions: readonly FixtureTopology[] = ['direct', 'express'];
