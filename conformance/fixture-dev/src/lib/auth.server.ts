import { randomBytes, timingSafeEqual } from 'node:crypto';
import { FIXTURE_DEV_ORIGIN } from './contract.ts';

const tokenKey = Symbol.for('float-view.fixture-dev.token');
type TokenGlobals = typeof globalThis & { [tokenKey]?: string };

export function launchToken(): string {
	const globals = globalThis as TokenGlobals;
	globals[tokenKey] ??= randomBytes(32).toString('base64url');
	return globals[tokenKey];
}

export function authorizeMutation(request: Request, expectedToken = launchToken()): void {
	if (request.method !== 'POST') throw new HttpActionError(405, 'Only POST mutations are allowed');
	if (request.headers.get('origin') !== FIXTURE_DEV_ORIGIN) {
		throw new HttpActionError(403, 'Unexpected mutation Origin');
	}
	const received = request.headers.get('x-fixture-token') ?? '';
	const expected = Buffer.from(expectedToken);
	const actual = Buffer.from(received);
	if (expected.length !== actual.length || !timingSafeEqual(expected, actual)) {
		throw new HttpActionError(403, 'Invalid fixture launch token');
	}
	const length = Number(request.headers.get('content-length') ?? 0);
	if (!Number.isFinite(length) || length < 0 || length > 65_536) {
		throw new HttpActionError(413, 'Fixture action body exceeds bound');
	}
	if (!request.headers.get('content-type')?.toLowerCase().startsWith('application/json')) {
		throw new HttpActionError(415, 'Fixture actions require application/json');
	}
}

export class HttpActionError extends Error {
	readonly status: number;

	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}
