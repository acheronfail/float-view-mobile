import type { Target } from '../src/suite.js';

export function externalTarget(name: string, environmentName: string): Target {
	let endpoint: { url: string; origin: string } | undefined;
	return {
		name,
		async start() {
			const value = process.env[environmentName];
			if (!value) throw new Error(`${environmentName} must contain the backend WebSocket URL`);
			const url = new URL(value);
			const origin = `${url.protocol === 'wss:' ? 'https:' : 'http:'}//${url.host}`;
			endpoint = { url: value, origin };
			return endpoint;
		},
		async endpoint() {
			if (!endpoint) throw new Error(`${name} backend is not running`);
			return endpoint;
		},
		async stop() {}
	};
}
