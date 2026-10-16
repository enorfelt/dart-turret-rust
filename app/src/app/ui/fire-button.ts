import { Component, computed, input, output, signal } from '@angular/core';
import { FireMode } from '../protocol/turret-protocol';
import { Icon } from './icon';

@Component({
  selector: 'app-fire-button',
  imports: [Icon],
  host: { '[class.compact]': 'compact()' },
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
      @if (compact()) {
        <app-icon name="target" />
      }
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

    :host(.compact) button {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 2px;
      width: 96px;
      height: 96px;
      min-height: 0;
      padding: 0 8px;
      border-radius: 50%;
      font-size: 1.125rem;
      line-height: 1;
    }

    :host(.compact) app-icon {
      font-size: 2rem;
    }
  `,
})
export class FireButton {
  readonly mode = input.required<FireMode>();
  readonly disabled = input(false);
  readonly compact = input(false);
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
