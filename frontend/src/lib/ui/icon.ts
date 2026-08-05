export const iconClasses = {
  'arrow-left': 'i-lucide-arrow-left',
  bluetooth: 'i-lucide-bluetooth',
  'ellipsis-vertical': 'i-lucide-ellipsis-vertical',
  'loader-circle': 'i-lucide-loader-circle',
  'map-pin': 'i-lucide-map-pin',
  route: 'i-lucide-route',
  settings: 'i-lucide-settings',
  signal: 'i-lucide-signal',
  'signal-high': 'i-lucide-signal-high',
  'signal-low': 'i-lucide-signal-low',
  'signal-medium': 'i-lucide-signal-medium',
  'triangle-alert': 'i-lucide-triangle-alert',
} as const;

export type IconName = keyof typeof iconClasses;
