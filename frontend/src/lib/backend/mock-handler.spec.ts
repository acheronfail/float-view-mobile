import { describe, expect, it } from 'vitest';
import { MockBackend } from '../../../backend-mock/api-v1';

describe('deterministic mock backend', () => {
	it('implements state and publishes full monotonic state changes', () => {
		const events: unknown[] = [];
		const backend = new MockBackend((event) => events.push(event));
		expect(backend.handle({ id: 1, method: 'state.get', params: {} })).toMatchObject({
			id: 1,
			result: { revision: 0 }
		});
		expect(backend.handle({ id: 2, method: 'bluetooth.scan', params: {} })).toEqual({
			id: 2,
			result: { accepted: true }
		});
		expect(events).toEqual([
			expect.objectContaining({
				event: 'state.changed',
				data: expect.objectContaining({ revision: 1 })
			})
		]);
	});

	it('rejects malformed, unknown, and stale mutations with stable errors', () => {
		const backend = new MockBackend();
		expect(backend.handlePayload('{')).toMatchObject({
			error: { code: 'invalid_request' }
		});
		expect(backend.handle({ id: 1, method: 'other', params: {} })).toMatchObject({
			error: { code: 'invalid_request' }
		});
		expect(
			backend.handle({
				id: 2,
				method: 'settings.update',
				params: { expectedRevision: 9, patch: {} }
			})
		).toMatchObject({ error: { code: 'conflict' } });
	});

	it('covers every mandatory method', () => {
		const backend = new MockBackend();
		const request = (id: number, method: string, params: Record<string, unknown> = {}) =>
			backend.handle({ id, method, params });
		request(1, 'bluetooth.scan');
		request(2, 'device.connect', { deviceId: 'mock-board' });
		expect(request(3, 'device.disconnect')).toMatchObject({ result: { accepted: true } });
		expect(request(4, 'boardProfile.get', { savedDeviceId: 'saved-board' })).toHaveProperty(
			'result'
		);
		expect(request(5, 'session.get', { sessionId: 'session-1' })).toHaveProperty('result');
		expect(request(6, 'session.getDiagnostics', { sessionId: 'session-1' })).toHaveProperty(
			'result'
		);
		expect(request(7, 'session.export', { sessionId: 'session-1', format: 'csv' })).toHaveProperty(
			'result'
		);
	});
});
