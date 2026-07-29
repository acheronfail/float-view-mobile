import { describe, expect, it } from 'vitest';
import { DEVELOPMENT_PORT, MOCK_BACKEND_ORIGIN, selectedBackendOrigin } from '../../../vite.config';

describe('Vite backend selection', () => {
	it('uses the stable mock and Vite ports by default', () => {
		expect(DEVELOPMENT_PORT).toBe(11002);
		expect(MOCK_BACKEND_ORIGIN).toBe('http://127.0.0.1:11003');
		expect(selectedBackendOrigin({})).toBe(MOCK_BACKEND_ORIGIN);
	});

	it('accepts only an explicit loopback HTTP origin', () => {
		expect(selectedBackendOrigin({ FLOAT_VIEW_DEV_BACKEND_ORIGIN: 'http://127.0.0.1:11010' })).toBe(
			'http://127.0.0.1:11010'
		);
		expect(() =>
			selectedBackendOrigin({ FLOAT_VIEW_DEV_BACKEND_ORIGIN: 'http://192.168.1.2:11010' })
		).toThrow('must be an http://127.0.0.1');
	});
});
