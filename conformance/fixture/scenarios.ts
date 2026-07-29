import { expressThor, type BoardConfiguration } from './boards.ts';

export type PhysicalScenarioName = 'identity' | 'ride' | 'unexpected-disconnect' | 'background';

export interface FixtureScenario {
	name: PhysicalScenarioName;
	board: BoardConfiguration;
	fragmentBytes: number;
	minimumTelemetryResponses: number;
	expectedTelemetryIntervalMs?: number;
	cadenceToleranceMs?: number;
	disconnectAfterTelemetry?: number;
	requiresMobileLifecycle: boolean;
}

export const fixtureScenarios: Record<PhysicalScenarioName, FixtureScenario> = {
	identity: {
		name: 'identity',
		board: expressThor,
		fragmentBytes: 244,
		minimumTelemetryResponses: 0,
		requiresMobileLifecycle: false
	},
	ride: {
		name: 'ride',
		board: expressThor,
		fragmentBytes: 20,
		minimumTelemetryResponses: 18,
		expectedTelemetryIntervalMs: 100,
		cadenceToleranceMs: 80,
		requiresMobileLifecycle: false
	},
	'unexpected-disconnect': {
		name: 'unexpected-disconnect',
		board: expressThor,
		fragmentBytes: 20,
		minimumTelemetryResponses: 6,
		disconnectAfterTelemetry: 4,
		requiresMobileLifecycle: false
	},
	background: {
		name: 'background',
		board: expressThor,
		fragmentBytes: 20,
		minimumTelemetryResponses: 8,
		requiresMobileLifecycle: true
	}
};

export const physicalScenarioMatrix = {
	macos: ['identity', 'ride', 'unexpected-disconnect'],
	android: ['identity', 'ride', 'unexpected-disconnect', 'background'],
	ios: ['identity', 'ride', 'unexpected-disconnect', 'background']
} as const satisfies Record<string, readonly PhysicalScenarioName[]>;
