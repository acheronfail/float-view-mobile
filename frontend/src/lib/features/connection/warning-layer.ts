/** Keep warnings on the dashboard; conceal them while any modal owns interaction. */
export function warningLayer(node: HTMLElement) {
  const owner = node.parentElement!;
  function update() {
    const blocked = [
      ...document.querySelectorAll<HTMLElement>('dialog[open], [role="dialog"], [role="alertdialog"]'),
    ].some((el) => !el.closest('[inert]') && el.getClientRects().length > 0 && !node.contains(el));
    if (node.inert !== blocked) node.inert = blocked;
    node.style.visibility = blocked ? 'hidden' : '';
    if (node.getAttribute('aria-hidden') !== String(blocked)) node.setAttribute('aria-hidden', String(blocked));
    // Preserve dashboard geometry while a panel is open, including during a hold.
    owner.style.setProperty('--live-warning-clearance', `${node.getBoundingClientRect().height + 8}px`);
  }
  const observer = new MutationObserver(update);
  observer.observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['open', 'inert', 'aria-hidden'],
  });
  const resize = new ResizeObserver(update);
  resize.observe(node);
  update();
  return {
    destroy() {
      observer.disconnect();
      resize.disconnect();
      owner.style.removeProperty('--live-warning-clearance');
    },
  };
}
