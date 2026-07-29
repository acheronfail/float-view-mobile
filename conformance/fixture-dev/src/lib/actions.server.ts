import {
	FixtureActionError,
	type FixtureCoordinator
} from './coordinator/coordinator.server.ts';
import {
	firmwareOptions,
	refloatOptions,
	topologyOptions,
	type FixtureAction,
	type FixtureConfiguration
} from './contract.ts';
import { authorizeMutation, HttpActionError } from './auth.server.ts';

export async function handleActionRequest(
	request: Request,
	coordinator: FixtureCoordinator,
	token?: string
): Promise<Response> {
	try {
		authorizeMutation(request, token);
		const text = await request.text();
		if (text.length > 4_096) throw new HttpActionError(413, 'Fixture action body exceeds bound');
		const action = parseAction(JSON.parse(text) as unknown);
		const snapshot = await coordinator.act(action);
		return json(snapshot);
	} catch (error) {
		if (error instanceof FixtureActionError) {
			return json({ error: error.message, snapshot: error.snapshot }, 409);
		}
		if (error instanceof HttpActionError) return json({ error: error.message }, error.status);
		return json({ error: message(error) }, error instanceof SyntaxError ? 400 : 400);
	}
}

export function parseAction(value: unknown): FixtureAction {
	if (!isRecord(value) || typeof value.kind !== 'string') throw new Error('Invalid fixture action');
	switch (value.kind) {
		case 'attach':
			if (typeof value.path !== 'string' || !value.path || value.path.length > 512) {
				throw new Error('Invalid serial path');
			}
			return { kind: 'attach', path: value.path };
		case 'detach':
		case 'power-on':
		case 'power-off':
		case 'start-riding':
		case 'stop-riding':
			return { kind: value.kind };
		case 'configure': {
			const configuration = value.configuration;
			if (!isRecord(configuration)) throw new Error('Invalid fixture configuration');
			if (
				typeof configuration.topology !== 'string' ||
				!(topologyOptions as readonly string[]).includes(configuration.topology) ||
				typeof configuration.firmware !== 'string' ||
				!(firmwareOptions as readonly string[]).includes(configuration.firmware) ||
				typeof configuration.refloat !== 'string' ||
				!(refloatOptions as readonly string[]).includes(configuration.refloat) ||
				typeof configuration.canId !== 'string' ||
				!/^(0|[1-9][0-9]{0,2})$/.test(configuration.canId)
			) {
				throw new Error('Invalid fixture configuration');
			}
			const canId = Number(configuration.canId);
			if (canId > 253) throw new Error('CAN ID must be from 0 through 253');
			return {
				kind: 'configure',
				configuration: {
					topology: configuration.topology as FixtureConfiguration['topology'],
					firmware: configuration.firmware as FixtureConfiguration['firmware'],
					refloat: configuration.refloat as FixtureConfiguration['refloat'],
					canId
				}
			};
		}
		default:
			throw new Error('Unknown fixture action');
	}
}

function json(value: unknown, status = 200) {
	return Response.json(value, {
		status,
		headers: {
			'cache-control': 'no-store',
			'content-security-policy': "default-src 'none'; frame-ancestors 'none'"
		}
	});
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function message(error: unknown) {
	return error instanceof Error ? error.message : String(error);
}
