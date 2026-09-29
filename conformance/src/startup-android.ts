import assert from 'node:assert/strict';
import { androidStartupPresentationTests } from '../targets/android.ts';

const output = androidStartupPresentationTests();
assert.match(output, /OK \(1 test\)/, `Native startup UI checks failed:\n${output}`);
assert.doesNotMatch(output, /FAILURES|INSTRUMENTATION_FAILED|Process crashed/);
console.log('Android startup canvas, retained/disposed resume, memory pressure and Activity recreation passed');
