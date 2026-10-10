/** Keep touch focus quiet until the user navigates with a keyboard. */
export function trackKeyboardNavigation() {
  const root = document.documentElement;
  const onKeyDown = (event: KeyboardEvent) => {
    // Navigation keys distinguish keyboard use from software-keyboard typing.
    if (!event.isComposing && ['Tab', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.key)) {
      root.dataset.keyboardNavigation = 'true';
    }
  };
  const onPointerDown = () => {
    delete root.dataset.keyboardNavigation;
  };
  // Capture before controls can stop propagation or move focus into a dialog.
  window.addEventListener('keydown', onKeyDown, true);
  window.addEventListener('pointerdown', onPointerDown, true);
  return () => {
    window.removeEventListener('keydown', onKeyDown, true);
    window.removeEventListener('pointerdown', onPointerDown, true);
    delete root.dataset.keyboardNavigation;
  };
}
