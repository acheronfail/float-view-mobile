import type {
	HelloParams,
	HelloResult,
	JsonRpcId,
	RpcErrorResponse,
	Snapshot
} from '../../api/generated/typescript/models.js';

type JsonRpcRequest = {
	jsonrpc?: unknown;
	id?: unknown;
	method?: unknown;
	params?: unknown;
};

export type ApiV1Session = {
	initialized: boolean;
};

export type ApiV1Response =
	| { jsonrpc: '2.0'; id: JsonRpcId; result: HelloResult | Snapshot }
	| RpcErrorResponse;

export const mockSnapshot: Snapshot = {
	revision: 0,
	bluetooth: {
		radio: 'powered_on',
		scanPermission: 'granted',
		connectPermission: 'granted'
	},
	devices: [],
	telemetrySubscriptions: [],
	loggingJobs: []
};

const helloResult: HelloResult = {
	api: { major: 1, minor: 0 },
	backend: {
		name: 'float-view-mock',
		version: '0.0.1',
		platform: 'mock'
	},
	capabilities: [
		{ name: 'system.snapshot', available: true },
		{
			name: 'device.connection',
			available: false,
			constraints: { reason: 'Not migrated to API v1 yet' }
		}
	],
	limits: {
		maxConnections: 1,
		minTelemetryIntervalMs: 200,
		maxMessageBytes: 65_536
	},
	snapshot: mockSnapshot
};

function error(
	id: JsonRpcId | null,
	code: number,
	kind: 'invalid_state' | 'unsupported' | 'protocol_error',
	message: string,
	recoverable = false
): RpcErrorResponse {
	return {
		jsonrpc: '2.0',
		id,
		error: {
			code,
			message,
			data: { kind, recoverable }
		}
	};
}

function isId(value: unknown): value is JsonRpcId {
	return (
		(typeof value === 'string' && value.length > 0 && value.length <= 128) ||
		(typeof value === 'number' && Number.isInteger(value))
	);
}

function isHelloParams(value: unknown): value is HelloParams {
	if (!value || typeof value !== 'object') return false;
	const params = value as Record<string, unknown>;
	if (!params.client || typeof params.client !== 'object') return false;
	if (!params.api || typeof params.api !== 'object') return false;
	const client = params.client as Record<string, unknown>;
	const api = params.api as Record<string, unknown>;
	return (
		typeof client.name === 'string' &&
		client.name.length > 0 &&
		typeof client.version === 'string' &&
		client.version.length > 0 &&
		typeof api.major === 'number' &&
		Number.isInteger(api.major) &&
		api.major >= 1 &&
		typeof api.minMinor === 'number' &&
		Number.isInteger(api.minMinor) &&
		api.minMinor >= 0 &&
		typeof api.maxMinor === 'number' &&
		Number.isInteger(api.maxMinor) &&
		api.maxMinor >= api.minMinor
	);
}

export function handleApiV1Request(session: ApiV1Session, input: unknown): ApiV1Response {
	if (!input || typeof input !== 'object') {
		return error(null, -32600, 'protocol_error', 'Invalid JSON-RPC request');
	}

	const request = input as JsonRpcRequest;
	const id = isId(request.id) ? request.id : null;
	if (request.jsonrpc !== '2.0' || id === null || typeof request.method !== 'string') {
		return error(id, -32600, 'protocol_error', 'Invalid JSON-RPC request');
	}

	switch (request.method) {
		case 'system.hello': {
			if (!isHelloParams(request.params)) {
				return error(id, -32602, 'protocol_error', 'Invalid system.hello parameters');
			}
			if (request.params.api.major !== 1) {
				return error(id, -32001, 'unsupported', 'Unsupported API major version');
			}
			if (request.params.api.minMinor > 0 || request.params.api.maxMinor < 0) {
				return error(id, -32001, 'unsupported', 'No compatible API minor version');
			}
			session.initialized = true;
			return { jsonrpc: '2.0', id, result: helloResult };
		}
		case 'system.getSnapshot':
			if (!session.initialized) {
				return error(
					id,
					-32000,
					'invalid_state',
					'system.hello must complete before other requests',
					true
				);
			}
			return { jsonrpc: '2.0', id, result: mockSnapshot };
		default:
			return error(id, -32601, 'unsupported', `Unknown method: ${request.method}`);
	}
}

export function handleApiV1Payload(session: ApiV1Session, payload: string): ApiV1Response {
	try {
		return handleApiV1Request(session, JSON.parse(payload));
	} catch {
		return error(null, -32700, 'protocol_error', 'Invalid JSON');
	}
}
