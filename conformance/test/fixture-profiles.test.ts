import assert from 'node:assert/strict';
import test from 'node:test';
import { composeBoard } from '../fixture/boards.ts';
import { firmwareVariants, type FirmwareVersion } from '../fixture/firmware.ts';
import { encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import {
	encodeRefloatTelemetry,
	refloatCodecs,
	type RefloatVersion
} from '../fixture/refloat.ts';
import {
	idleTelemetry,
	InteractiveTelemetrySource,
	type TelemetrySample
} from '../fixture/telemetry.ts';
import type { FixtureTopology } from '../fixture/topology.ts';
import { VescSimulator } from '../fixture/simulator.ts';

const firmwareVersions = Object.keys(firmwareVariants) as FirmwareVersion[];
const refloatVersions = Object.keys(refloatCodecs) as RefloatVersion[];
const topologies: FixtureTopology[] = ['direct', 'express'];

test('firmware variants retain THOR300 identity and version-specific tail shape', () => {
	assert.equal(firmwareVariants['6.02'].payload.length, firmwareVariants['6.05'].payload.length);
	assert.equal(firmwareVariants['6.06'].payload.length, firmwareVariants['6.05'].payload.length + 4);
	for (const version of firmwareVersions) {
		const payload = firmwareVariants[version].payload;
		assert.deepEqual([...payload.slice(0, 3)], [0, 6, Number(version.slice(-2))]);
		assert.equal(new TextDecoder().decode(payload.slice(3, 10)), 'THOR300');
	}
});

for (const firmware of firmwareVersions) {
	for (const refloat of refloatVersions) {
		for (const topology of topologies) {
			test(`${topology} composes THOR300 ${firmware} with Refloat ${refloat}`, () => {
				const telemetry = new InteractiveTelemetrySource(7);
				telemetry.start();
				const simulator = new VescSimulator(
					composeBoard({
						firmware,
						refloat,
						topology,
						canId: 42,
						telemetry,
						requestPolicy: 'interactive'
					})
				);
				const prefix = topology === 'express' ? [34, 42] : [];
				const firmwareResponse =
					topology === 'express'
						? single(simulator, [34, 42, 0])
						: single(simulator, [0]);
				const firmwarePayload = decode(firmwareResponse.frame);
				assert.deepEqual([...firmwarePayload.slice(0, 3)], [
					0,
					6,
					Number(firmware.slice(-2))
				]);
				if (topology === 'express') {
					assert.equal(decode(single(simulator, [0]).frame)[2], 5);
					assert.deepEqual([...decode(single(simulator, [62]).frame)], [62, 42]);
				}

				const info = decode(single(simulator, [...prefix, 36, 101, 0, 2, 0]).frame);
				assert.equal(info[3], refloat === '1.0.0' ? 10 : 2);
				if (refloat !== '1.0.0') {
					assert.deepEqual([...info.slice(25, 28)], refloat.split('.').map(Number));
					assert.equal(
						single(simulator, [...prefix, 36, 101, 32]).request.kind,
						'refloat-layout'
					);
				}
				const packet = refloat === '1.0.0' ? 10 : 31;
				assert.equal(
					single(simulator, [...prefix, 36, 101, packet]).request.telemetryPhase,
					'riding'
				);
				assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(6))), []);
			});
		}
	}
}

test('interactive Express forwarding ignores another CAN identity without falling through', () => {
	const simulator = new VescSimulator(
		composeBoard({
			firmware: '6.06',
			refloat: '1.2.3',
			topology: 'express',
			canId: 42
		})
	);
	assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(34, 41, 0))), []);
	assert.match(simulator.diagnostics.at(-1) ?? '', /Unknown CAN target 41/);
});

test('seeded telemetry is smooth, reproducible, correlated, and stops immediately', () => {
	const left = new InteractiveTelemetrySource(123);
	const right = new InteractiveTelemetrySource(123);
	assert.equal(left.start(), true);
	assert.equal(left.start(), false);
	right.start();
	const samples = Array.from({ length: 12 }, () => left.next());
	const repeated = Array.from({ length: 12 }, () => right.next());
	assert.deepEqual(samples, repeated);
	for (let index = 1; index < samples.length; index += 1) {
		assert.ok(Math.abs(samples[index]!.speedKph - samples[index - 1]!.speedKph) < 2);
		assert.ok(samples[index]!.erpm > samples[index]!.speedKph * 200);
		assert.ok(samples[index]!.packVoltageV < idleTelemetry.packVoltageV);
	}
	assert.equal(left.stop(), true);
	assert.equal(left.stop(), false);
	assert.deepEqual(left.next(), idleTelemetry);
});

test('all Refloat encoders retain equivalent normalized core state', () => {
	const sample: TelemetrySample = {
		rideState: 'riding',
		boardState: 3,
		footpadState: 3,
		speedKph: 21.6,
		erpm: 5000,
		dutyCycle: 0.5,
		motorCurrentA: 24,
		batteryCurrentA: 10,
		packVoltageV: 62,
		controllerTempC: 34,
		motorTempC: 32
	};
	for (const version of refloatVersions) {
		const payload = encodeRefloatTelemetry(version, sample);
		assert.equal(payload[0], 36);
		assert.equal(payload[1], 101);
		assert.equal(version === '1.0.0' ? payload[10] : payload[9], version === '1.0.0' ? 1 : 3);
		assert.equal(
			version === '1.0.0' ? payload[11] : payload[10]! >> 6,
			version === '1.0.0' ? 2 : 3
		);
	}
});

function single(simulator: VescSimulator, payload: number[]) {
	const responses = simulator.receive(encodeFrame(Uint8Array.from(payload)));
	assert.equal(responses.length, 1);
	return responses[0]!;
}

function decode(frame: Uint8Array) {
	const payloads = new FrameAssembler().append(frame);
	assert.equal(payloads.length, 1);
	return payloads[0]!;
}
