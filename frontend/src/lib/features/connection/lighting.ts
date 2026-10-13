import type { ActiveConnection, LiveWidgetField } from '$lib/backend/contract';
export type LightingControl = 'headlights' | 'leds';
export function isLighting(field: LiveWidgetField): field is LightingControl {
  return field === 'headlights' || field === 'leds';
}
export function lightingMenuActions(connection: ActiveConnection) {
  if (!connection.refloatControls?.lightingAvailable || connection.refloatControls.status === 'unavailable') return [];
  return (['leds', 'headlights'] as const).map((control) => ({
    control,
    label: control === 'leds' ? 'LEDs' : 'Headlights',
    ...lightingBox(connection, control),
  }));
}
export function lightingBox(connection: ActiveConnection | null | undefined, control: LightingControl) {
  const controls = connection?.refloatControls;
  const available = !!controls?.lightingAvailable && controls.status !== 'unavailable';
  const checked = available ? (control === 'headlights' ? controls.headlightsEnabled : controls.ledsEnabled) : null;
  const pending = controls?.pendingAction === control;
  const disabled =
    !available ||
    checked == null ||
    connection?.state !== 'ready' ||
    controls?.status !== 'ready' ||
    !controls.writable ||
    controls.pendingAction !== null ||
    controls.flywheelEnabled === true;
  return {
    checked,
    pending,
    disabled,
    status: !available ? 'Unavailable' : checked == null ? 'Unknown' : pending ? 'Pending' : checked ? 'On' : 'Off',
  };
}
