export type FocusTarget = { focus(): void };

export function wrappedDialogFocus<T>(active: T | null, controls: readonly T[], shiftKey: boolean): T | null {
  const first = controls[0];
  const last = controls.at(-1);
  if (!first || !last) return null;
  if (shiftKey && active === first) return last;
  if (!shiftKey && active === last) return first;
  return null;
}

export function restoreDialogFocus(target: FocusTarget | undefined): void {
  target?.focus();
}
