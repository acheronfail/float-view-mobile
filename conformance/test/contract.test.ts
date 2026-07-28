import assert from 'node:assert/strict';
import test from 'node:test';
import { mandatoryMethods, runConformance } from '../src/suite.js';
import { mockTarget } from '../targets/mock.js';

test('mandatory contract has one concise method set', () => {
	assert.equal(mandatoryMethods.length, 13);
	assert.equal(new Set(mandatoryMethods).size, mandatoryMethods.length);
});

test('mock passes canonical black-box conformance', async () => {
	const result = await runConformance(mockTarget);
	assert.equal(result.target, 'mock');
});
