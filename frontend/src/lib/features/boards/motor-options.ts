// Float Hub's published motor table, updated 2026-10-12:
// https://pev.dev/t/float-hub-documentation/2406
// These identify a board's motor; selecting one does not write motor configuration.
export const motorOptions = [
  { value: '', label: 'Select motor', disabled: true },
  { value: 'hypercore_xr', label: 'Hypercore', group: 'Future Motion' },
  { value: 'hypercore_v2', label: 'Hypercore v2 (S-Series / XRC)', group: 'Future Motion' },
  { value: 'sidewinder', label: 'Sidewinder', group: 'The Float Life' },
  { value: 'superflux_mk1', label: 'SuperFlux Mk1', group: 'Fungineers' },
  { value: 'superflux_mk2_hs', label: 'SuperFlux Mk2 HS', group: 'Fungineers' },
  { value: 'superflux_mk2_ht', label: 'SuperFlux Mk2 HT', group: 'Fungineers' },
  { value: 'superflux_mk3_hs', label: 'SuperFlux Mk3 HS', group: 'Fungineers' },
  { value: 'superflux_mk3_ht', label: 'SuperFlux Mk3 HT', group: 'Fungineers' },
  { value: 'superflux_5_hs', label: 'SuperFlux 5" HS', group: 'Fungineers' },
  { value: 'superflux_5_ht', label: 'SuperFlux 5" HT', group: 'Fungineers' },
  { value: 'cannoncore_v1', label: 'CannonCore V1', group: 'Floatwheel' },
  { value: 'cannoncore_v2', label: 'CannonCore V2 (ADV1)', group: 'Floatwheel' },
  { value: 'cannoncore_v3', label: 'CannonCore V3 (ADV2)', group: 'Floatwheel' },
  { value: 'babycore', label: 'BabyCore (Atom)', group: 'Floatwheel' },
  { value: 'custom', label: 'Custom / other', group: 'Other' },
];
