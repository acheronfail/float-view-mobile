import type { HostLayout } from '../../../frontend/src/lib/app/host-layout.ts';

// Sanitized geometry from native CI run 40460393322 (Xcode 27.1). No device identifiers.
export const closedSideLayout = {
  version: 1,
  edge: 'right',
  bounds: {
    width: 466,
    height: 678,
  },
  safeArea: {
    bottom: 34,
    left: 0,
    right: 84,
    top: 0,
  },
  occlusions: [
    {
      height: 37,
      width: 37,
      x: 399.6666666666667,
      y: 29.333333333333332,
    },
    {
      height: 174,
      width: 84,
      x: 382,
      y: 0,
    },
  ],
} satisfies HostLayout;
