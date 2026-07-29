export type Activation = () => void | Promise<void>;

export function activateFromPointer(event: PointerEvent, action: Activation) {
	if (!event.isPrimary || event.button !== 0) return;
	event.preventDefault();
	void action();
}

export function activateFromKeyboardOrAssistiveClick(event: MouseEvent, action: Activation) {
	if (event.detail !== 0) return;
	void action();
}
