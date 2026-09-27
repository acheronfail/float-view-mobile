// Shared host launch/setup lives in native tests. Assertions stay canonical.
import { runConformance } from './suite.ts';
import { externalTarget } from '../targets/external.ts';
const target = externalTarget('macos', 'FLOAT_VIEW_SHARED_WS_URL');
await runConformance(target);
console.log('Shared-server canonical conformance passed');
