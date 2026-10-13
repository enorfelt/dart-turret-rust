import { Component, ElementRef, inject, output, signal } from '@angular/core';
import { Deflection } from '../aim/aim-controller';

const KEY_DEFLECTION = 0.6;
const ARROWS: Record<string, Deflection> = {
  ArrowLeft: { x: -1, y: 0 },
  ArrowRight: { x: 1, y: 0 },
  ArrowUp: { x: 0, y: 1 },
  ArrowDown: { x: 0, y: -1 },
};

@Component({
  selector: 'app-joystick',
  host: {
    role: 'application',
    tabindex: '0',
    'aria-label': 'Aim. Drag the stick, or hold the arrow keys.',
    '[style.--x]': 'deflection().x',
    '[style.--y]': 'deflection().y',
    '(pointerdown)': 'press($event)',
    '(pointermove)': 'move($event)',
    '(pointerup)': 'letGo($event)',
    '(pointercancel)': 'letGo($event)',
    '(keydown)': 'keyDown($event)',
    '(keyup)': 'keyUp($event)',
    '(blur)': 'releaseKeys()',
  },
  template: `<div class="knob"></div>`,
  styles: `
    :host {
      --travel: 34%;
      position: relative;
      display: grid;
      place-items: center;
      aspect-ratio: 1;
      width: min(100%, 270px, 56dvh);
      border: 2px solid var(--panel-raised);
      border-radius: 50%;
      background:
        linear-gradient(var(--panel-raised), var(--panel-raised)) center / 2px 70% no-repeat,
        linear-gradient(var(--panel-raised), var(--panel-raised)) center / 70% 2px no-repeat,
        var(--panel);
      touch-action: none;
      user-select: none;
    }

    .knob {
      width: 38%;
      aspect-ratio: 1;
      border-radius: 50%;
      background: var(--ink);
      box-shadow: inset 0 -6px 0 rgb(0 0 0 / 0.25);
      transform: translate(
        calc(var(--x) * var(--travel) * 2.6),
        calc(var(--y) * var(--travel) * -2.6)
      );
    }
  `,
})
export class Joystick {
  readonly engaged = output<void>();
  readonly deflected = output<Deflection>();
  readonly released = output<void>();

  protected readonly deflection = signal<Deflection>({ x: 0, y: 0 });
  private readonly host: HTMLElement = inject(ElementRef).nativeElement;
  private pointerId: number | null = null;
  private readonly heldKeys = new Set<string>();

  protected press(event: PointerEvent): void {
    if (this.pointerId !== null || this.heldKeys.size > 0) return;
    this.pointerId = event.pointerId;
    this.host.setPointerCapture(event.pointerId);
    this.engaged.emit();
    this.follow(event);
  }

  protected move(event: PointerEvent): void {
    if (event.pointerId === this.pointerId) this.follow(event);
  }

  protected letGo(event: PointerEvent): void {
    if (event.pointerId !== this.pointerId) return;
    this.pointerId = null;
    this.center();
  }

  protected keyDown(event: KeyboardEvent): void {
    if (!(event.key in ARROWS) || this.pointerId !== null) return;
    event.preventDefault();
    if (this.heldKeys.has(event.key)) return;
    if (this.heldKeys.size === 0) this.engaged.emit();
    this.heldKeys.add(event.key);
    this.followKeys();
  }

  protected keyUp(event: KeyboardEvent): void {
    if (!this.heldKeys.delete(event.key)) return;
    if (this.heldKeys.size === 0) {
      this.center();
    } else {
      this.followKeys();
    }
  }

  protected releaseKeys(): void {
    if (this.heldKeys.size === 0) return;
    this.heldKeys.clear();
    this.center();
  }

  private follow(event: PointerEvent): void {
    const rect = this.host.getBoundingClientRect();
    const radius = rect.width / 2;
    const x = (event.clientX - rect.left - radius) / radius;
    const y = (rect.top + radius - event.clientY) / radius;
    const length = Math.hypot(x, y);
    this.emit(length > 1 ? { x: x / length, y: y / length } : { x, y });
  }

  private followKeys(): void {
    const sum = [...this.heldKeys].reduce(
      (total, key) => ({ x: total.x + ARROWS[key].x, y: total.y + ARROWS[key].y }),
      { x: 0, y: 0 },
    );
    this.emit({ x: sum.x * KEY_DEFLECTION, y: sum.y * KEY_DEFLECTION });
  }

  private center(): void {
    this.emit({ x: 0, y: 0 });
    this.released.emit();
  }

  private emit(deflection: Deflection): void {
    this.deflection.set(deflection);
    this.deflected.emit(deflection);
  }
}
