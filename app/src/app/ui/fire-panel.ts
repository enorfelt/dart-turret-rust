import { Component, computed, inject, signal } from '@angular/core';
import { FormField, form, hidden, max, min } from '@angular/forms/signals';
import { FireMode } from '../protocol/turret-protocol';
import { Turret } from '../turret/turret';
import { FireButton } from './fire-button';

export interface FireSettings {
  mode: FireMode;
  throttle: number;
  shots: number;
}

export const MAX_BURST_SHOTS = 10;

@Component({
  selector: 'app-fire-panel',
  imports: [FormField, FireButton],
  template: `
    <fieldset class="modes">
      <legend>Mode</legend>
      <label [class.selected]="fireForm.mode().value() === 'single'">
        <input type="radio" value="single" [formField]="fireForm.mode" />
        Single
      </label>
      <label [class.selected]="fireForm.mode().value() === 'burst'">
        <input type="radio" value="burst" [formField]="fireForm.mode" />
        Burst
      </label>
      <label [class.selected]="fireForm.mode().value() === 'auto'">
        <input type="radio" value="auto" [formField]="fireForm.mode" />
        Auto
      </label>
    </fieldset>

    <label class="throttle">
      <span class="caption">Flywheel speed</span>
      <output>{{ fireForm.throttle().value() }} %</output>
      <input type="range" step="1" [formField]="fireForm.throttle" />
    </label>

    @if (!fireForm.shots().hidden()) {
      <div class="shots" role="group" aria-labelledby="shots-caption">
        <span class="caption" id="shots-caption">Darts per burst</span>
        <button
          type="button"
          aria-label="Fewer darts"
          [disabled]="settings().shots <= 1"
          (click)="stepShots(-1)"
        >
          −
        </button>
        <output aria-live="polite">{{ fireForm.shots().value() }}</output>
        <button
          type="button"
          aria-label="More darts"
          [disabled]="settings().shots >= maxShots"
          (click)="stepShots(1)"
        >
          +
        </button>
      </div>
    }

    <app-fire-button
      [mode]="fireForm.mode().value()"
      [disabled]="!canFire()"
      (fire)="fire()"
      (cease)="turret.ceaseFire()"
    />
  `,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      gap: clamp(8px, 3dvh, 16px);
    }

    .caption,
    legend {
      font-family: var(--font-label);
      font-size: 1.125rem;
      font-weight: 500;
      color: var(--ink-muted);
    }

    .modes {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      margin: 0;
      padding: 0;
      border: 0;
    }

    legend {
      margin-bottom: 6px;
      padding: 0;
    }

    .modes label {
      display: grid;
      place-items: center;
      min-height: 44px;
      border: 2px solid var(--panel-raised);
      font-family: var(--font-label);
      font-size: 1.375rem;
      font-weight: 700;
      cursor: pointer;
    }

    .modes label:first-of-type {
      border-radius: var(--radius-control) 0 0 var(--radius-control);
    }

    .modes label:last-of-type {
      border-radius: 0 var(--radius-control) var(--radius-control) 0;
    }

    .modes label + label {
      border-left: 0;
    }

    .modes label.selected {
      background: var(--ink);
      border-color: var(--ink);
      color: var(--body);
    }

    .modes label:has(input:focus-visible) {
      outline: 3px solid var(--hazard);
      outline-offset: 2px;
    }

    .modes input {
      position: absolute;
      opacity: 0;
      pointer-events: none;
    }

    .throttle {
      display: grid;
      grid-template-columns: 1fr auto;
      gap: 4px 12px;
    }

    .throttle output,
    .shots output {
      font-family: var(--font-label);
      font-size: 1.375rem;
      font-weight: 700;
      font-variant-numeric: tabular-nums;
    }

    .throttle input {
      grid-column: 1 / -1;
      width: 100%;
      height: 32px;
      margin: 0;
      accent-color: var(--ink);
    }

    .shots {
      display: grid;
      grid-template-columns: 1fr 48px 3ch 48px;
      align-items: center;
      gap: 8px;
      text-align: center;
    }

    .shots button {
      height: 48px;
      border: 2px solid var(--panel-raised);
      border-radius: var(--radius-control);
      background: transparent;
      font-size: 1.5rem;
    }

    .shots button:disabled {
      opacity: 0.4;
    }

    .shots .caption {
      text-align: left;
    }
  `,
})
export class FirePanel {
  protected readonly turret = inject(Turret);
  protected readonly maxShots = MAX_BURST_SHOTS;

  protected readonly settings = signal<FireSettings>({ mode: 'single', throttle: 50, shots: 3 });
  protected readonly fireForm = form(this.settings, (field) => {
    min(field.throttle, 1);
    max(field.throttle, 100);
    min(field.shots, 1);
    max(field.shots, MAX_BURST_SHOTS);
    hidden(field.shots, ({ valueOf }) => valueOf(field.mode) !== 'burst');
  });

  protected readonly canFire = computed(() => this.turret.canFire() && this.fireForm().valid());

  protected fire(): void {
    const { mode, throttle, shots } = this.settings();
    this.turret.fire(mode, throttle, mode === 'burst' ? shots : 1);
  }

  protected stepShots(delta: number): void {
    this.settings.update((settings) => ({
      ...settings,
      shots: Math.min(Math.max(settings.shots + delta, 1), MAX_BURST_SHOTS),
    }));
  }
}
