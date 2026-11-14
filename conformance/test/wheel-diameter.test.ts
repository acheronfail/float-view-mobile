import assert from 'node:assert/strict';
import { test } from 'node:test';
import { applyWheelDiameterWrite, motorConfigurationForVersion } from '../fixture/firmware.ts';

test('fixture motor writes preserve every other byte and reject unsafe or unrelated edits', () => {
  for (const [major, minor, len, pos] of [
    [6, 2, 482, 457],
    [6, 5, 478, 447],
    [6, 6, 484, 453],
    [7, 0, 489, 458],
  ]) {
    const layout = motorConfigurationForVersion(major!, minor!)!;
    assert.equal(layout.bytes.length, len);
    assert.equal(layout.diameterOffset, pos);
    const request = layout.bytes.slice();
    request[0] = 13;
    new DataView(request.buffer).setFloat32(pos!, 0.2688, false);
    const result = applyWheelDiameterWrite(layout.bytes, request, pos!, true)!;
    assert.ok(result);
    assert.equal(result[0], 14);
    for (let i = 1; i < result.length; i++) if (i < pos! || i >= pos! + 4) assert.equal(result[i], layout.bytes[i]);
    assert.equal(applyWheelDiameterWrite(layout.bytes, request, pos!, false), null);
    request[40] = request[40]! ^ 1;
    assert.equal(applyWheelDiameterWrite(layout.bytes, request, pos!, true), null);
    request[40] = request[40]! ^ 1;
    for (const meters of [0.1, 0.6]) {
      new DataView(request.buffer).setFloat32(pos!, meters, false);
      assert.ok(applyWheelDiameterWrite(layout.bytes, request, pos!, true));
    }
    new DataView(request.buffer).setFloat32(pos!, NaN, false);
    assert.equal(applyWheelDiameterWrite(layout.bytes, request, pos!, true), null);
  }
  assert.equal(motorConfigurationForVersion(99, 99), null);
});
