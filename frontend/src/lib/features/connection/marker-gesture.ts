type Point = { x: number; y: number; time: number };
type Pointer = Pick<PointerEvent, 'pointerId' | 'isPrimary' | 'button' | 'clientX' | 'clientY' | 'timeStamp'>;
export class MarkerGesture {
  private down: (Point & { id: number }) | null = null;
  private first: Point | null = null;
  cancel() {
    this.down = null;
    this.first = null;
  }
  observePointer(event: Pick<PointerEvent, 'pointerId'>) {
    if (this.down && event.pointerId !== this.down.id) this.cancel();
  }
  leave(event: Pick<PointerEvent, 'pointerType'>) {
    // Touch emits pointerleave after every lift; only an exit during a held
    // touch cancels the pair. Mouse/hover exits still cancel between taps.
    if (event.pointerType !== 'touch' || this.down) this.cancel();
  }
  start(event: Pointer) {
    if (!event.isPrimary || event.button !== 0 || this.down) {
      this.cancel();
      return;
    }
    this.down = { id: event.pointerId, x: event.clientX, y: event.clientY, time: event.timeStamp };
  }
  move(event: Pointer) {
    if (
      this.down &&
      (event.pointerId !== this.down.id || Math.hypot(event.clientX - this.down.x, event.clientY - this.down.y) > 12)
    )
      this.cancel();
  }
  end(event: Pointer) {
    this.move(event);
    const down = this.down;
    this.down = null;
    if (
      !down ||
      !event.isPrimary ||
      event.button !== 0 ||
      event.pointerId !== down.id ||
      event.timeStamp - down.time > 300
    ) {
      this.cancel();
      return false;
    }
    const point = { x: event.clientX, y: event.clientY, time: event.timeStamp };
    const paired =
      this.first &&
      point.time - this.first.time <= 350 &&
      Math.hypot(point.x - this.first.x, point.y - this.first.y) <= 24;
    this.first = paired ? null : point;
    return Boolean(paired);
  }
}
