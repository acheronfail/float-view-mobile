import { getMockBootstrap, startMockServer, stopMockServer } from '../../frontend/backend-mock/index.ts';
import type { Target } from '../src/suite.ts';

export const MOCK_BACKEND_PORT = 11003;

export const mockTarget: Target = {
  name: 'mock',
  async start() {
    const port = await startMockServer(MOCK_BACKEND_PORT);
    const bootstrap = getMockBootstrap(port);
    currentEndpoint = { url: bootstrap.webSocketUrl, origin: bootstrap.origin };
    return currentEndpoint;
  },
  async endpoint() {
    if (!currentEndpoint) throw new Error('Mock backend is not running');
    return currentEndpoint;
  },
  stop: stopMockServer,
};

let currentEndpoint: { url: string; origin: string } | undefined;
