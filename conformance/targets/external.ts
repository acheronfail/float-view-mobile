import type { Target } from '../src/suite.js';

export function externalTarget(name: string, environmentName: string): Target {
	return {
		name,
		async start() {
			const value = process.env[environmentName];
			if (!value) throw new Error(`${environmentName} must contain the backend WebSocket URL`);
			const url = new URL(value);
			const origin = `${url.protocol === 'wss:' ? 'https:' : 'http:'}//${url.host}`;
			return { url: value, origin };
		},
		async stop() {}
	};
}
