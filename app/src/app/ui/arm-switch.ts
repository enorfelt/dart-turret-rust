import { Component, DestroyRef, inject, input, signal } from '@angular/core';
import { Turret } from '../turret/turret';
import { Icon } from './icon';

const ARM_HOLD_MS = 1000;

@Component({
  selector: 'app-arm-switch',
  imports: [Icon],
  host: { '[style.--hold-duration]': 'holdDuration', '[class.compact]': 'compact()' },
  template: `
    @if (turret.armed()) {
      <button type="button" class="disarm" (click)="turret.disarm()">
        @if (compact()) {
          <app-icon name="power" />
        }
        Disarm
      </button>
    } @else if (!turret.armSwitchOn()) {
      <button type="button" class="arm" disabled>
        @if (compact()) {
          <app-icon name="power" />
        }
        Arm switch off
      </button>
    } @else {
      <button
        type="button"
        class="arm"
        [class.holding]="holding()"
        (pointerdown)="hold()"
        (pointerup)="cancel()"
        (pointerleave)="cancel()"
        (pointercancel)="cancel()"
        (keydown.enter)="hold()"
        (keyup.enter)="cancel()"
        (keydown.space)="$event.preventDefault(); hold()"
        (keyup.space)="cancel()"
        (contextmenu)="$event.preventDefault()"
      >
        <span class="fill" aria-hidden="true"></span>
        @if (compact()) {
          <app-icon name="power" />
        }
        <span class="label">{{ holding() ? 'Keep holding' : 'Hold to arm' }}</span>
      </button>
    }
  `,
  styles: `
    button {
      position: relative;
      overflow: hidden;
      width: 100%;
      min-height: 64px;
      border-radius: var(--radius-control);
      font-family: var(--font-label);
      font-size: 1.75rem;
      font-weight: 700;
      user-select: none;
      -webkit-user-select: none;
    }

    .arm {
      border: 2px solid var(--hazard);
      background: transparent;
      color: var(--hazard);
    }

    .arm:disabled {
      border-color: var(--panel-raised);
      color: var(--ink-muted);
    }

    .fill {
      position: absolute;
      inset: 0;
      background: var(--hazard);
      transform: scaleX(0);
      transform-origin: left;
    }

    .holding .fill {
      animation: fill var(--hold-duration) linear forwards;
    }

    .holding .label {
      color: var(--body);
    }

    .label {
      position: relative;
    }

    .disarm {
      border: 0;
      background: var(--hazard);
      color: var(--body);
      font-size: 2.25rem;
      min-height: clamp(64px, 18dvh, 88px);
    }

    :host(.compact) button {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 6px;
      min-height: 44px;
      padding: 0 12px;
      font-size: 1.125rem;
    }

    :host(.compact) .disarm {
      min-height: 52px;
      font-size: 1.5rem;
    }

    app-icon {
      position: relative;
    }

    .holding app-icon {
      color: var(--body);
    }

    @keyframes fill {
      to {
        transform: scaleX(1);
      }
    }
  `,
})
export class ArmSwitch {
  readonly compact = input(false);
  protected readonly turret = inject(Turret);
  protected readonly holding = signal(false);
  protected readonly holdDuration = `${ARM_HOLD_MS}ms`;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor() {
    inject(DestroyRef).onDestroy(() => clearTimeout(this.timer));
  }

  protected hold(): void {
    if (this.holding()) return;
    this.holding.set(true);
    this.timer = setTimeout(() => {
      this.holding.set(false);
      if (this.turret.armSwitchOn()) this.turret.arm();
    }, ARM_HOLD_MS);
  }

  protected cancel(): void {
    clearTimeout(this.timer);
    this.holding.set(false);
  }
}
