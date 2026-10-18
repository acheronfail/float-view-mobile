import { boardAppearanceColours, type BoardAppearance } from './contract.ts';

export function validBoardAppearance(value: unknown): value is BoardAppearance {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const v = value as Record<string, unknown>;
  const styles = { rail: 6, tyre: 2, fender: 3, hooks: 3 };
  const colours = ['leftTop', 'leftBottom', 'rightTop', 'rightBottom', 'rubber', 'rim', 'fenders', 'hookColour'];
  return (
    Object.keys(v).length === 12 &&
    Object.entries(styles).every(
      ([key, max]) => Number.isInteger(v[key]) && Number(v[key]) >= 1 && Number(v[key]) <= max
    ) &&
    colours.every((key) => boardAppearanceColours.some(([, colour]) => colour === v[key])) &&
    !(v.hooks === 2 && v.fender === 1)
  );
}
