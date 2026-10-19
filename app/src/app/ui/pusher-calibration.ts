import { Component, computed, inject, signal } from '@angular/core';
import { FormField, form, max, min, required } from '@angular/forms/signals';
import { PUSHER_MAX_US, PUSHER_MIN_US } from '../protocol/turret-protocol';
import { Turret } from '../turret/turret';

const STEPS = [-50, -10, 10, 50] as const;

@Component({
  selector: 'app-pusher-calibration',
  imports: [FormField],
  template: `
    <details>
      <summary>Pusher calibration</summary>

      <p class="status" aria-live="polite">{{ status() }}</p>
      @if (!turret.armSwitchOn()) {
        <p class="warning">Arm switch off: the servo has no power.</p>
      }

      <form (submit)="$event.preventDefault(); hold()">
        <label>
          <span class="caption">Pulse ({{ minUs }}–{{ maxUs }} µs)</span>
          <input type="number" inputmode="numeric" step="10" [formField]="pulseForm.pulseUs" />
        </label>
        <button type="submit" [disabled]="!pulseForm().valid()">Hold</button>
      </form>

      <div class="steps" role="group" aria-label="Step the pulse">
        @for (step of steps; track step) {
          <button type="button" [disabled]="!pulseForm().valid()" (click)="nudge(step)">
            {{ step > 0 ? '+' + step : '−' + -step }}
          </button>
        }
      </div>

      <button
        type="button"
        class="release"
        [disabled]="turret.heldPusherUs() === null"
        (click)="turret.releasePusher()"
      >
        Release
      </button>
      <p class="hint">Release at once if the servo buzzes: it is stalled against a stop.</p>
    </details>
  `,
  styles: `
    summary {
      font-family: var(--font-label);
      font-size: 1.125rem;
      font-weight: 500;
      color: var(--ink-muted);
      cursor: pointer;
    }

    details[open] {
      display: flex;
      flex-direction: column;
      gap: 10px;
    }

    p {
      margin: 0;
    }

    .status {
      font-family: var(--font-label);
      font-size: 1.25rem;
      font-weight: 500;
      font-variant-numeric: tabular-nums;
    }

    .warning {
      color: var(--hazard);
    }

    .hint,
    .caption {
      color: var(--ink-muted);
    }

    form {
      display: grid;
      grid-template-columns: 1fr auto;
      align-items: end;
      gap: 8px;
    }

    label {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    input {
      min-height: 44px;
      padding: 0 10px;
      border: 2px solid var(--panel-raised);
      border-radius: var(--radius-control);
      background: var(--panel);
      color: var(--ink);
      font: inherit;
      font-variant-numeric: tabular-nums;
    }

    .steps {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 8px;
    }

    button {
      min-height: 44px;
      padding: 0 14px;
      border: 2px solid var(--panel-raised);
      border-radius: var(--radius-control);
      background: transparent;
      font-family: var(--font-label);
      font-size: 1.125rem;
      font-weight: 700;
      font-variant-numeric: tabular-nums;
    }

    button:disabled {
      opacity: 0.4;
    }
  `,
})
export class PusherCalibration {
  protected readonly turret = inject(Turret);
  protected readonly steps = STEPS;
  protected readonly minUs = PUSHER_MIN_US;
  protected readonly maxUs = PUSHER_MAX_US;

  private readonly pulse = signal<{ pulseUs: number | null }>({ pulseUs: null });
  protected readonly pulseForm = form(this.pulse, (field) => {
    required(field.pulseUs);
    min(field.pulseUs, PUSHER_MIN_US);
    max(field.pulseUs, PUSHER_MAX_US);
  });

  protected readonly status = computed(() => {
    const held = this.turret.heldPusherUs();
    return held === null ? 'Not held: the pusher is retracted' : `Holding at ${held} µs`;
  });

  protected hold(): void {
    const { pulseUs } = this.pulse();
    if (pulseUs !== null && this.pulseForm().valid()) {
      this.turret.holdPusher(Math.round(pulseUs));
    }
  }

  protected nudge(step: number): void {
    this.pulse.update(({ pulseUs }) => ({
      pulseUs: Math.min(Math.max((pulseUs ?? PUSHER_MIN_US) + step, PUSHER_MIN_US), PUSHER_MAX_US),
    }));
    this.hold();
  }
}
