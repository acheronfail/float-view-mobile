import { describe, expect, it, vi } from 'vitest';
import {
	activateFromKeyboardOrAssistiveClick,
	activateFromPointer,
	isKeyboardOrAssistiveClick,
	isPrimaryPointerActivation
} from './activation';

describe('cross-platform control activation', () => {
	it('accepts only the primary pointer', () => {
		expect(isPrimaryPointerActivation({ button: 0, isPrimary: true })).toBe(true);
		expect(isPrimaryPointerActivation({ button: 1, isPrimary: true })).toBe(false);
		expect(isPrimaryPointerActivation({ button: 0, isPrimary: false })).toBe(false);
	});

	it('keeps synthesized keyboard and assistive clicks without repeating pointer clicks', () => {
		expect(isKeyboardOrAssistiveClick({ detail: 0 })).toBe(true);
		expect(isKeyboardOrAssistiveClick({ detail: 1 })).toBe(false);
	});

	it('dispatches each supported activation path once', () => {
		const action = vi.fn();
		const preventDefault = vi.fn();

		activateFromPointer(
			{ button: 0, isPrimary: true, preventDefault } as unknown as PointerEvent,
			action
		);
		activateFromKeyboardOrAssistiveClick({ detail: 1 } as MouseEvent, action);
		activateFromKeyboardOrAssistiveClick({ detail: 0 } as MouseEvent, action);

		expect(preventDefault).toHaveBeenCalledOnce();
		expect(action).toHaveBeenCalledTimes(2);
	});
});
