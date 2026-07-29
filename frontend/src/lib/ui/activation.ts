export type Activation = () => void | Promise<void>;

export function isPrimaryPointerActivation(
	event: Pick<PointerEvent, 'button' | 'isPrimary'>
): boolean {
	return event.isPrimary && event.button === 0;
}

export function isKeyboardOrAssistiveClick(event: Pick<MouseEvent, 'detail'>): boolean {
	return event.detail === 0;
}

export function activateFromPointer(event: PointerEvent, action: Activation): void {
	if (!isPrimaryPointerActivation(event)) return;
	event.preventDefault();
	void action();
}

export function activateFromKeyboardOrAssistiveClick(event: MouseEvent, action: Activation): void {
	if (!isKeyboardOrAssistiveClick(event)) return;
	void action();
}
