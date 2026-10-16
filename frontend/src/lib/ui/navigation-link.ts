import { goto } from '$app/navigation';
import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from './activation';

/** Keep native links, with release activation that survives live snapshot updates. */
export function navigationLink(node: HTMLAnchorElement, { scrollable = false } = {}) {
  const modified = (event: MouseEvent) => event.metaKey || event.ctrlKey || event.shiftKey || event.altKey;
  const navigate = () => goto(node.getAttribute('href')!);
  const pointer = (event: PointerEvent) => {
    if (!modified(event)) activateFromPointer(event, navigate, { scrollable });
  };
  const click = (event: MouseEvent) => {
    if (modified(event)) return;
    event.preventDefault();
    activateFromKeyboardOrAssistiveClick(event, navigate);
  };
  node.addEventListener('pointerdown', pointer);
  node.addEventListener('click', click);
  return {
    destroy() {
      node.removeEventListener('pointerdown', pointer);
      node.removeEventListener('click', click);
    },
  };
}
