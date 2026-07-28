import assert from 'node:assert/strict';
import test from 'node:test';
import { expressThor } from '../fixture/boards.js';
import { crc16, encodeFrame, FrameAssembler } from '../fixture/frame.js';
import { led, parseEvent } from '../fixture/serial.js';
import { VescSimulator } from '../fixture/simulator.js';

test('frame assembler handles fragmentation and validates CRC', () => {
	const frame = encodeFrame(Uint8Array.of(0));
	assert.equal(crc16(Uint8Array.of(0)), 0);
	const assembler = new FrameAssembler();
	assert.deepEqual(assembler.append(frame.slice(0, 2)), []);
	assert.deepEqual(assembler.append(frame.slice(2)), [Uint8Array.of(0)]);
});

test('simulator responds to read-only requests and rejects mutation', () => {
	const simulator = new VescSimulator(expressThor);
	assert.equal(simulator.receive(encodeFrame(Uint8Array.of(0))).length, 1);
	assert.throws(
		() => simulator.receive(encodeFrame(Uint8Array.of(6))),
		/Unsafe or unsupported/
	);
});

test('serial events and diagnostic LED conventions are bounded', () => {
	assert.deepEqual(parseEvent('READY'), { kind: 'READY' });
	assert.deepEqual(parseEvent('WRITE 00AFF0'), {
		kind: 'WRITE',
		bytes: Uint8Array.of(0, 0xaf, 0xf0)
	});
	assert.equal(led.off, '000000');
	assert.throws(() => parseEvent('WRITE xyz'));
});
