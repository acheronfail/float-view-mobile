import type { BoardAppearance } from '$lib/backend/contract';

export const appearanceParts = ['rail', 'tyre', 'fender', 'hooks'] as const;
export type AppearancePart = (typeof appearanceParts)[number];
export type AppearanceColour = Exclude<keyof BoardAppearance, AppearancePart>;
export const partLabels = { rail: 'Rails', tyre: 'Tyre', fender: 'Fender', hooks: 'Hooks' };
export const styleCounts = { rail: 6, tyre: 2, fender: 3, hooks: 3 };
export const colourAreas: Record<AppearancePart, { key: AppearanceColour; label: string }[]> = {
  rail: [
    { key: 'leftTop', label: 'Left · top' },
    { key: 'rightTop', label: 'Right · top' },
    { key: 'leftBottom', label: 'Left · bottom' },
    { key: 'rightBottom', label: 'Right · bottom' },
  ],
  tyre: [
    { key: 'rubber', label: 'Tyre' },
    { key: 'rim', label: 'Rim' },
  ],
  fender: [{ key: 'fenders', label: 'Fender' }],
  hooks: [{ key: 'hookColour', label: 'Hooks' }],
};

export function cycleAppearance(value: BoardAppearance, part: AppearancePart, direction: number): BoardAppearance {
  const next = { ...value, [part]: ((value[part] - 1 + direction + styleCounts[part]) % styleCounts[part]) + 1 };
  if (next.hooks === 2 && next.fender === 1) {
    if (part === 'hooks') next.fender = 2;
    else next.hooks = 1;
  }
  return next as BoardAppearance;
}

const profiles = [
  [
    [30, 124],
    [154, 124],
    [190, 124],
    [290, 124],
    [326, 124],
    [450, 124],
  ],
  [
    [30, 118],
    [154, 140],
    [190, 124],
    [290, 124],
    [326, 140],
    [450, 118],
  ],
  [
    [30, 105],
    [154, 143],
    [190, 124],
    [290, 124],
    [326, 143],
    [450, 105],
  ],
  [
    [30, 114],
    [159, 135],
    [197, 124],
    [279, 124],
    [319, 139],
    [450, 115],
  ],
  [
    [30, 119],
    [157, 135],
    [195, 124],
    [288, 124],
    [330, 136],
    [450, 101],
  ],
  [
    [30, 124],
    [154, 124],
    [190, 124],
    [290, 124],
    [326, 124],
    [450, 95],
  ],
] as const;
export function appearanceShapes(s: BoardAppearance) {
  const p = profiles[s.rail - 1];
  const points = (v: readonly (readonly number[])[]) => v.map((p) => p.join(',')).join(' ');
  const half = (y: number) => points([[0, y], [480, y], [480, p[5][1]], ...[...p].reverse(), [0, p[0][1]]]);
  const polar = (r: number, a: number) => `${(240 + r * Math.cos(a)).toFixed(2)} ${(124 + r * Math.sin(a)).toFixed(2)}`;
  let gear = 'M' + polar(66, -Math.PI / 12);
  for (let i = 0; i < 12; i++) {
    const a = (i * Math.PI) / 6;
    gear += ` A66 66 0 0 1 ${polar(66, a - 0.14)} L${polar(74, a - 0.1)} L${polar(74, a + 0.1)} L${polar(66, a + 0.14)} A66 66 0 0 1 ${polar(66, a + Math.PI / 12)}`;
  }
  const band = (a: number, b: number) =>
    `M${polar(98, a)} A98 98 0 0 1 ${polar(98, b)} L${polar(84, b)} A84 84 0 0 0 ${polar(84, a)} Z`;
  const tip = 'M-5 0 V-33 Q-5 -43 5 -43 H34 Q43 -43 43 -34 V-27 H9 V0 Z';
  const angle = (a: readonly number[], b: readonly number[]) => (Math.atan2(b[1] - a[1], b[0] - a[0]) * 180) / Math.PI;
  return {
    rail: points(p),
    top: half(0),
    bottom: half(248),
    tyre: s.tyre === 2 ? gear + ' Z' : 'M174 124 A66 66 0 1 0 306 124 A66 66 0 1 0 174 124 Z',
    fenders:
      s.fender === 3
        ? [band(Math.PI, 2 * Math.PI)]
        : s.fender === 2
          ? [band(Math.PI, Math.PI + 0.72), band(2 * Math.PI - 0.72, 2 * Math.PI)]
          : [],
    hooks:
      s.hooks === 3
        ? [
            { path: tip, transform: `translate(${p[0].join(' ')}) rotate(${angle(p[0], p[1])})` },
            { path: tip, transform: `translate(${p[5].join(' ')}) rotate(${angle(p[4], p[5])}) scale(-1 1)` },
          ]
        : s.hooks === 2
          ? [
              { path: 'M165 122 L153 94 H139 V81 H162 L181 118 Z', transform: 'translate(-40 0)' },
              { path: 'M315 122 L327 94 H341 V81 H318 L299 118 Z', transform: 'translate(40 0)' },
            ]
          : [],
  };
}

export function swatchText(colour: string): string {
  const v = colour
    .slice(1)
    .match(/../g)!
    .map((v) => parseInt(v, 16) / 255)
    .map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return v[0] * 0.2126 + v[1] * 0.7152 + v[2] * 0.0722 > 0.179 ? '#17191d' : '#ffffff';
}
