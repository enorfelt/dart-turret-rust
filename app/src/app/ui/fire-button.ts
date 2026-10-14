import { Component, computed, input, output, signal } from '@angular/core';
import { FireMode } from '../protocol/turret-protocol';

@Component({
  selector: 'app-fire-button',
  template: `
    <button
      type="button"
      [class.held]="held()"
      [disabled]="disabled() && !held()"
      (pointerdown)="press($event)"
      (pointerup)="letGo()"
      (pointercancel)="letGo()"
      (lostpointercapture)="letGo()"
      (blur)="letGo()"
      (keydown.enter)="press()"
      (keyup.enter)="letGo()"
      (keydown.space)="$event.preventDefault(); press()"
      (keyup.space)="letGo()"
      (contextmenu)="$event.preventDefault()"
    >
      {{ label() }}
    </button>
  `,
  styles: `
    button {
      width: 100%;
      min-height: clamp(72px, 12dvh, 120px);
      border: 0;
      border-radius: 60px;
      background: var(--fire);
      color: #fff;
      font-family: var(--font-label);
      font-size: 2.5rem;
      font-weight: 700;
      box-shadow: inset 0 -8px 0 var(--fire-pressed);
      touch-action: none;
      user-select: none;
      -webkit-user-select: none;
    }

    .held {
      background: var(--fire-pressed);
      box-shadow: none;
      transform: translateY(4px);
    }

    button:disabled {
      background: var(--panel);
      color: var(--ink-muted);
      box-shadow: none;
    }
  `,
})
export class FireButton {
  readonly mode = input.required<FireMode>();
  readonly disabled = input(false);
  readonly fire = output<void>();
  readonly cease = output<void>();

  protected readonly held = signal(false);
  protected readonly label = computed(() => (this.mode() === 'auto' ? 'Hold to fire' : 'Fire'));

  protected press(event?: PointerEvent): void {
    if (this.disabled() || this.held()) return;
    if (event) (event.currentTarget as Element).setPointerCapture(event.pointerId);
    this.held.set(true);
    this.fire.emit();
  }

  protected letGo(): void {
    if (!this.held()) return;
    this.held.set(false);
    if (this.mode() === 'auto') this.cease.emit();
  }
}
