import { getMockBootstrap, startMockServer, stopMockServer } from '../../frontend/backend-mock/index.js';
import type { Target } from '../src/suite.js';

export const mockTarget: Target = {
	name: 'mock',
	async start() {
		const port = await startMockServer(0);
		const bootstrap = getMockBootstrap(port);
		return { url: bootstrap.webSocketUrl, origin: bootstrap.origin };
	},
	stop: stopMockServer
};
