import { describe, expect, it } from 'vitest';
import { handleApiV1Payload, handleApiV1Request } from '../../../backend-mock/api-v1';

const helloRequest = {
	jsonrpc: '2.0',
	id: 'hello-1',
	method: 'system.hello',
	params: {
		client: { name: 'contract-test', version: '1.0.0' },
		api: { major: 1, minMinor: 0, maxMinor: 0 }
	}
};

describe('mock API v1 handler', () => {
	it('requires hello before snapshot requests', () => {
		const response = handleApiV1Request(
			{ initialized: false },
			{
				jsonrpc: '2.0',
				id: 1,
				method: 'system.getSnapshot',
				params: {}
			}
		);

		expect('error' in response && response.error.data.kind).toBe('invalid_state');
	});

	it('negotiates v1.0 and returns an initial snapshot', () => {
		const session = { initialized: false };
		const response = handleApiV1Request(session, helloRequest);

		expect(session.initialized).toBe(true);
		expect('result' in response && response.result).toMatchObject({
			api: { major: 1, minor: 0 },
			backend: { platform: 'mock' },
			snapshot: { revision: 0 }
		});
	});

	it('rejects an unsupported major version with a stable error', () => {
		const response = handleApiV1Request(
			{ initialized: false },
			{
				...helloRequest,
				params: {
					...helloRequest.params,
					api: { major: 2, minMinor: 0, maxMinor: 0 }
				}
			}
		);

		expect('error' in response && response.error).toMatchObject({
			code: -32001,
			data: { kind: 'unsupported' }
		});
	});

	it('returns the authoritative snapshot after hello', () => {
		const session = { initialized: false };
		handleApiV1Request(session, helloRequest);
		const response = handleApiV1Request(session, {
			jsonrpc: '2.0',
			id: 2,
			method: 'system.getSnapshot',
			params: {}
		});

		expect('result' in response && response.result).toMatchObject({
			revision: 0,
			devices: [],
			loggingJobs: []
		});
	});

	it('uses the standard parse-error code for invalid JSON', () => {
		const response = handleApiV1Payload({ initialized: false }, '{');

		expect('error' in response && response.error).toMatchObject({
			code: -32700,
			data: { kind: 'protocol_error' }
		});
	});
});
