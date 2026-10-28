import { describe, expect, it, vi } from 'vitest';
import { FootpadFaultDetector, footpadFaultIcon, footpadFaultLabel, loadFootpadFaults } from './footpad-faults';

describe('footpad faults at speed', () => {
  it('uses exact states, absolute speed, and one marker per episode across sample pages', () => {
    const detector = new FootpadFaultDetector([
      { elapsedMs: 0, sequence: 0, state: 'none' },
      { elapsedMs: 500, sequence: 5, state: 'left' },
      { elapsedMs: 700, sequence: 7, state: 'right' },
      { elapsedMs: 800, sequence: 8, state: 'unknown' },
    ]);
    [null, 2, 2.1, 20, 0, -10, -20, 30, 30].forEach((speed, i) => detector.push(i * 100, i, speed));
    expect(detector.faults).toEqual([
      { elapsedMs: 200, state: 'none' },
      { elapsedMs: 500, state: 'left' },
      { elapsedMs: 700, state: 'right' },
    ]);
  });
  it('does not assume an initial pad state or use a future transition at the same timestamp', () => {
    const detector = new FootpadFaultDetector([{ elapsedMs: 100, sequence: 2, state: 'none' }]);
    detector.push(0, 0, 30);
    detector.push(100, 1, 30);
    expect(detector.faults).toEqual([]);
    detector.push(100, 2, 30);
    detector.push(200, 3, null);
    detector.push(300, 4, 30);
    detector.push(20_000, 5, 30);
    expect(detector.faults.map((fault) => fault.elapsedMs)).toEqual([100, 300, 20_000]);
  });
  it('shows the inactive side red and active side green with an accessible description', () => {
    expect(footpadFaultIcon('none').match(/#ef4444/g)).toHaveLength(2);
    expect(footpadFaultIcon('left')).toMatch(/fill="#22c55e".*fill="#ef4444"/);
    expect(footpadFaultIcon('right')).toMatch(/fill="#ef4444".*fill="#22c55e"/);
    expect(footpadFaultLabel({ elapsedMs: 2000, state: 'left' })).toContain('Right footpad inactive; left active');
  });

  it.each(['none', 'left', 'right'] as const)('excludes %s footpad episodes until known flywheel exit', (state) => {
    const detector = new FootpadFaultDetector([
      { elapsedMs: 0, sequence: 0, state },
      { elapsedMs: 0, sequence: 0, mode: 'flywheel' },
      { elapsedMs: 200, sequence: 2, mode: 'unknown' },
      { elapsedMs: 400, sequence: 4, mode: 'normal' },
    ]);
    for (let i = 0; i < 4; i++) detector.push(i * 100, i, -20);
    expect(detector.faults).toEqual([]);
    detector.push(400, 4, -20);
    detector.push(500, 5, -20);
    expect(detector.faults).toEqual([{ elapsedMs: 400, state }]);
  });

  it('orders mode transitions by source sample sequence when timestamps match', () => {
    const detector = new FootpadFaultDetector([
      { elapsedMs: 0, sequence: 0, state: 'none' },
      { elapsedMs: 100, sequence: 2, mode: 'flywheel' },
      { elapsedMs: 100, sequence: 4, mode: 'handTest' },
    ]);
    detector.push(0, 0, 20);
    detector.push(100, 1, 20);
    detector.push(100, 2, 20);
    detector.push(100, 3, 20);
    expect(detector.faults).toEqual([{ elapsedMs: 0, state: 'none' }]);
    detector.push(100, 4, 20);
    expect(detector.faults).toEqual([
      { elapsedMs: 0, state: 'none' },
      { elapsedMs: 100, state: 'none' },
    ]);
  });

  it('breaks episodes around flywheel even without retained speed samples inside the interval', () => {
    const detector = new FootpadFaultDetector([
      { elapsedMs: 0, sequence: 0, state: 'left' },
      { elapsedMs: 100, sequence: 1, mode: 'flywheel' },
      { elapsedMs: 200, sequence: 2, mode: 'normal' },
    ]);
    detector.push(0, 0, 20);
    detector.push(300, 3, 20);
    expect(detector.faults).toEqual([
      { elapsedMs: 0, state: 'left' },
      { elapsedMs: 300, state: 'left' },
    ]);
  });
});

it('paginates transitions and exact samples without duplicating an episode at a page boundary', async () => {
  const loadEvents = vi
    .fn()
    .mockResolvedValueOnce({
      page: { sequence: [0], elapsedMs: [0], sourceSampleSequence: [0], kinds: ['footpadState'], values: ['none'] },
      nextCursor: 'events-2',
    })
    .mockResolvedValueOnce({
      page: { sequence: [1], elapsedMs: [300], sourceSampleSequence: [3], kinds: ['footpadState'], values: ['right'] },
      nextCursor: null,
    });
  const loadSamples = vi
    .fn()
    .mockResolvedValueOnce({
      sequence: [0, 1],
      elapsedMs: [0, 100],
      values: { speedKph: [0, 20] },
      nextCursor: 'samples-2',
    })
    .mockResolvedValueOnce({
      sequence: [2, 3],
      elapsedMs: [200, 300],
      values: { speedKph: [20, 20] },
      nextCursor: null,
    });
  const controller = { loadEvents, loadSamples } as unknown as Parameters<typeof loadFootpadFaults>[0];
  const metadata = { eventCount: 2, storedSampleCount: 4, summary: { totalElapsedMs: 300 } } as Parameters<
    typeof loadFootpadFaults
  >[1];
  expect(await loadFootpadFaults(controller, metadata)).toEqual([
    { elapsedMs: 100, state: 'none' },
    { elapsedMs: 300, state: 'right' },
  ]);
  expect(loadEvents.mock.calls[1]![0]).toMatchObject({ cursor: 'events-2', limit: 500 });
  expect(loadSamples.mock.calls[0]![0]).toMatchObject({ cursor: null, fields: ['speedKph'] });
  expect(loadSamples.mock.calls[1]![0]).toMatchObject({ cursor: 'samples-2' });
  await expect(loadFootpadFaults(controller, { ...metadata, storedSampleCount: 1_000_001 })).rejects.toThrow(
    'too large'
  );
});

it('loads mode transitions across event and sample pages to exclude flywheel markers', async () => {
  const loadEvents = vi
    .fn()
    .mockResolvedValueOnce({
      page: {
        sequence: [0, 1],
        elapsedMs: [0, 200],
        sourceSampleSequence: [0, 2],
        kinds: ['footpadState', 'packageMode'],
        values: ['none', 'flywheel'],
      },
      nextCursor: 'events-2',
    })
    .mockResolvedValueOnce({
      page: {
        sequence: [2, 3, 4],
        elapsedMs: [300, 300, 400],
        sourceSampleSequence: [3, 3, 4],
        kinds: ['footpadState', 'packageMode', 'packageMode'],
        values: ['left', 'unrecognised', 'normal'],
      },
      nextCursor: null,
    });
  const loadSamples = vi
    .fn()
    .mockResolvedValueOnce({
      sequence: [1, 2, 3],
      elapsedMs: [100, 200, 300],
      values: { speedKph: [20, 20, 20] },
      nextCursor: 'samples-2',
    })
    .mockResolvedValueOnce({
      sequence: [4, 5],
      elapsedMs: [400, 500],
      values: { speedKph: [20, 20] },
      nextCursor: null,
    });
  const controller = { loadEvents, loadSamples } as unknown as Parameters<typeof loadFootpadFaults>[0];
  const metadata = { eventCount: 5, storedSampleCount: 5, summary: { totalElapsedMs: 500 } } as Parameters<
    typeof loadFootpadFaults
  >[1];
  expect(await loadFootpadFaults(controller, metadata)).toEqual([
    { elapsedMs: 100, state: 'none' },
    { elapsedMs: 400, state: 'left' },
  ]);
  expect(loadEvents).toHaveBeenCalledTimes(2);
  expect(loadSamples).toHaveBeenCalledTimes(2);
});
