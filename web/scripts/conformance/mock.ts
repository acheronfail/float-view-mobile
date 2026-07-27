import WebSocket from 'ws';
import { startMockServer, stopMockServer } from '../../backend-mock/index.js';
import { connectApiClient } from '../../src/lib/api/client.js';
import { runSystemConformance, type ConformanceClient } from '../../../conformance/src/system.ts';
import type { ApiMethod } from '../../../api/generated/typescript/models.js';

const port = await startMockServer(0);
const url = `ws://127.0.0.1:${port}/api/v1/ws`;

try {
	const results = await runSystemConformance(async (): Promise<ConformanceClient> => {
		const client = await connectApiClient(
			url,
			2_000,
			(socketUrl) => new WebSocket(socketUrl) as unknown as globalThis.WebSocket
		);
		return {
			request(method, params) {
				return client.request(method as ApiMethod, params as never);
			},
			close() {
				client.close();
			}
		};
	});

	for (const result of results) {
		console.log(`PASS ${result.name} (${result.durationMs.toFixed(1)}ms)`);
	}
	console.log(`Mock backend passed ${results.length} system conformance scenarios.`);
} finally {
	await stopMockServer();
}
