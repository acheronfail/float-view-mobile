export type FixtureTopology = 'direct' | 'express';

export interface DirectTopology {
	kind: 'direct';
}

export interface ExpressTopology {
	kind: 'express';
	canId: number;
}

export type TopologyStrategy = DirectTopology | ExpressTopology;

export const directTopology: DirectTopology = { kind: 'direct' };

export function expressTopology(canId = 1): ExpressTopology {
	if (!Number.isInteger(canId) || canId < 0 || canId > 253) {
		throw new Error('CAN ID must be an integer from 0 through 253');
	}
	return { kind: 'express', canId };
}
