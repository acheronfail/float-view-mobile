import type { FixtureAction, FixtureSnapshot, SerialPortOption } from '../contract.ts';

export class FixtureControlClient {
	private readonly token: string;

	constructor(token: string) {
		this.token = token;
	}

	async state(): Promise<FixtureSnapshot> {
		return this.get('/api/state');
	}

	async ports(): Promise<SerialPortOption[]> {
		return this.get('/api/ports');
	}

	async action(action: FixtureAction | Record<string, unknown>): Promise<FixtureSnapshot> {
		const response = await fetch('/api/action', {
			method: 'POST',
			headers: {
				'content-type': 'application/json',
				'x-fixture-token': this.token
			},
			body: JSON.stringify(action)
		});
		const value = (await response.json()) as {
			error?: string;
			snapshot?: FixtureSnapshot;
		};
		if (!response.ok) {
			const error = new Error(value.error ?? `Fixture action failed (${response.status})`);
			Object.assign(error, { snapshot: value.snapshot });
			throw error;
		}
		return value as FixtureSnapshot;
	}

	private async get<T>(path: string): Promise<T> {
		const response = await fetch(path, { headers: { accept: 'application/json' } });
		if (!response.ok) throw new Error(`Fixture request failed (${response.status})`);
		return (await response.json()) as T;
	}
}
