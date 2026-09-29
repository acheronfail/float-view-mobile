import { describe, expect, it } from 'vitest';
import { MarkerGesture } from './marker-gesture';
const event = (time: number, patch: Partial<PointerEvent> = {}) => ({
  pointerId: 1,
  isPrimary: true,
  button: 0,
  clientX: 100,
  clientY: 100,
  timeStamp: time,
  ...patch,
});
function tap(g: MarkerGesture, time: number) {
  g.start(event(time));
  return g.end(event(time + 50));
}
describe('marker gesture', () => {
  it('consumes pairs and ignores a single or third tap', () => {
    const g = new MarkerGesture();
    expect(tap(g, 0)).toBe(false);
    expect(tap(g, 200)).toBe(true);
    expect(tap(g, 300)).toBe(false);
    expect(tap(g, 500)).toBe(true);
  });
  it('rejects slow, long, distant, moved, cancelled and non-primary taps', () => {
    const g = new MarkerGesture();
    tap(g, 0);
    expect(tap(g, 500)).toBe(false);
    g.cancel();
    tap(g, 0);
    g.start(event(100));
    expect(g.end(event(450))).toBe(false);
    tap(g, 500);
    g.start(event(600));
    g.move(event(610, { clientX: 120 }));
    expect(g.end(event(620))).toBe(false);
    tap(g, 800);
    g.start(event(900, { clientX: 140 }));
    expect(g.end(event(950, { clientX: 140 }))).toBe(false);
    g.cancel();
    tap(g, 1000);
    g.start(event(1100, { isPrimary: false }));
    expect(g.end(event(1150))).toBe(false);
    tap(g, 1200);
    g.start(event(1300, { button: 2 }));
    expect(g.end(event(1350, { button: 2 }))).toBe(false);
  });
  it('rejects simultaneous pointers', () => {
    const g = new MarkerGesture();
    tap(g, 0);
    g.start(event(100));
    g.start(event(110, { pointerId: 2, isPrimary: false }));
    expect(g.end(event(150))).toBe(false);
  });
});

it('preserves a touch pair across lifts but cancels exits and outside multitouch', () => {
  const g = new MarkerGesture();
  tap(g, 0);
  g.leave({ pointerType: 'touch' });
  expect(tap(g, 200)).toBe(true);
  tap(g, 400);
  g.leave({ pointerType: 'mouse' });
  expect(tap(g, 600)).toBe(false);
  g.start(event(700));
  g.leave({ pointerType: 'touch' });
  expect(g.end(event(750))).toBe(false);
  tap(g, 800);
  g.start(event(900));
  g.observePointer({ pointerId: 2 });
  expect(g.end(event(950))).toBe(false);
});
