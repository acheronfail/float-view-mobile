import { expressThor, type BoardConfiguration } from './boards.js';

export interface FixtureScenario {
	name: string;
	board: BoardConfiguration;
	repetitions: number;
	fragmentBytes?: number;
	disconnectAfterRequests?: number;
	background?: boolean;
}

export const fixtureScenarios: FixtureScenario[] = [
	{ name: 'identity', board: expressThor, repetitions: 1 },
	{ name: 'ride', board: expressThor, repetitions: 10, fragmentBytes: 20 },
	{
		name: 'unexpected-disconnect',
		board: expressThor,
		repetitions: 2,
		disconnectAfterRequests: 8
	},
	{ name: 'background', board: expressThor, repetitions: 2, background: true }
];
