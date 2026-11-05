import type { Page } from 'playwright';

/** Exercise the same guarded Back acknowledgement as iOS, with a bounded test deadline. */
export async function dispatchEdgeBack(
  page: Page,
  interactive = true
): Promise<{ consumed: boolean; navigated: boolean }> {
  return page.evaluate(
    (interactive) =>
      new Promise((resolve, reject) => {
        const timeout = setTimeout(() => reject(new Error('Edge Back acknowledgement timed out')), 9000);
        let consumed = false;
        const event = new CustomEvent('floatview:back', {
          cancelable: true,
          detail: {
            source: 'edge',
            interactive,
            complete: (navigated: boolean) => {
              clearTimeout(timeout);
              // Dismissal and eligibility guards can acknowledge synchronously during dispatch.
              queueMicrotask(() => resolve({ consumed, navigated }));
            },
          },
        });
        consumed = !window.dispatchEvent(event);
      }),
    interactive
  );
}
