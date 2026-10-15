import { devices } from 'playwright';

/** CSS pixels, using the installed Playwright device registry. Native hosts use
 * the full screen; browser cases retain the smaller browser-content viewport. */
function device(
  id: string,
  label: string,
  preset: string,
  group: 'compact' | 'phones' | 'large' | 'foldables',
  browser = false
) {
  const descriptor = devices[preset] as (typeof devices)[string] & { screen?: { width: number; height: number } };
  if (!descriptor) throw new Error(`Missing Playwright device: ${preset}`);
  return { id, label, group, viewport: browser ? descriptor.viewport : (descriptor.screen ?? descriptor.viewport) };
}

export const liveViewports = [
  device('iphone-se', 'iPhone SE (compact)', 'iPhone SE', 'compact'),
  { id: 'android-small', label: 'Compact Android', group: 'compact', viewport: { width: 360, height: 640 } },
  device('iphone-8', 'iPhone 8 / SE 2–3 size', 'iPhone 8', 'compact'),
  device('galaxy-s9', 'Galaxy S9+', 'Galaxy S9+', 'compact'),
  device('galaxy-s8', 'Galaxy S8', 'Galaxy S8', 'compact'),
  device('iphone-browser', 'iPhone 15 Pro · browser controls', 'iPhone 15 Pro', 'compact', true),
  device('iphone-mini', 'iPhone 13 mini', 'iPhone 13 Mini', 'phones'),
  device('iphone-13', 'iPhone 13 / 14 size', 'iPhone 13', 'phones'),
  device('iphone-15-pro', 'iPhone 15 Pro', 'iPhone 15 Pro', 'phones'),
  device('iphone-16-pro', 'iPhone 16 Pro', 'iPhone 16 Pro', 'phones'),
  device('pixel-7', 'Pixel 7', 'Pixel 7', 'phones'),
  device('iphone-16-max', 'iPhone 16 Pro Max', 'iPhone 16 Pro Max', 'phones'),
  device('pixel-browser', 'Pixel 7 · browser controls', 'Pixel 7', 'large', true),
  device('ipad-mini', 'iPad mini', 'iPad Mini', 'large'),
  device('ipad-pro', 'iPad Pro 11', 'iPad Pro 11', 'large'),
  { id: 'se-landscape', label: 'Compact phone · landscape', group: 'large', viewport: { width: 568, height: 320 } },
  { id: 'iphone-landscape', label: 'iPhone 13 · landscape', group: 'large', viewport: { width: 844, height: 390 } },
  { id: 'pixel-landscape', label: 'Pixel 7 · landscape', group: 'large', viewport: { width: 915, height: 412 } },
  // Shape coverage rather than claims about a particular foldable product.
  { id: 'square-compact', label: 'Compact square', group: 'foldables', viewport: { width: 600, height: 600 } },
  { id: 'fold-portrait', label: 'Unfolded · portrait', group: 'foldables', viewport: { width: 673, height: 841 } },
  { id: 'square-medium', label: 'Medium square', group: 'foldables', viewport: { width: 720, height: 720 } },
  { id: 'fold-landscape', label: 'Unfolded · landscape', group: 'foldables', viewport: { width: 841, height: 673 } },
  { id: 'fold-wide', label: 'Wide unfolded', group: 'foldables', viewport: { width: 960, height: 720 } },
  { id: 'square-large', label: 'Large square', group: 'foldables', viewport: { width: 1024, height: 1024 } },
] as const;
