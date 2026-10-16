import { Component, computed, inject, input, signal } from '@angular/core';
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

const MODES: readonly FireMode[] = ['single', 'burst', 'auto'];
const MODE_LABELS: Record<FireMode, string> = { single: 'Single', burst: 'Burst', auto: 'Auto' };

@Component({
  selector: 'app-fire-panel',
  imports: [FormField, FireButton],
  host: { '[class.compact]': 'compact()' },
  template: `
    @if (compact()) {
      <button type="button" class="cycle" (click)="nextMode()">
        <span class="cycle-caption">Mode</span>
        <span class="cycle-value">{{ modeLabel() }}</span>
      </button>
    } @else {
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
    }

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
      [compact]="compact()"
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

    :host(.compact) {
      display: grid;
      grid-template-columns: auto 96px;
      gap: 8px;
    }

    :host(.compact) > :not(.throttle) {
      grid-column: 2;
    }

    :host(.compact) .caption {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip-path: inset(50%);
      white-space: nowrap;
    }

    .cycle {
      display: flex;
      flex-direction: column;
      align-items: center;
      min-height: 48px;
      padding: 2px 0 4px;
      border: 2px solid var(--panel-raised);
      border-radius: var(--radius-control);
      background: var(--panel);
      font-family: var(--font-label);
      line-height: 1.1;
    }

    .cycle-caption {
      font-size: 0.75rem;
      color: var(--ink-muted);
    }

    .cycle-value {
      font-size: 1.25rem;
      font-weight: 700;
    }

    :host(.compact) .throttle {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 4px;
      grid-column: 1;
      grid-row: 1 / span 3;
    }

    :host(.compact) .throttle output {
      font-size: 1rem;
    }

    :host(.compact) .throttle input {
      flex: 1;
      width: 28px;
      height: auto;
      min-height: 96px;
      writing-mode: vertical-lr;
      direction: rtl;
    }

    :host(.compact) .shots {
      grid-template-columns: 32px 1fr 32px;
      gap: 4px;
    }

    :host(.compact) .shots output {
      font-size: 1.125rem;
    }

    :host(.compact) .shots button {
      height: 36px;
      font-size: 1.25rem;
    }
  `,
})
export class FirePanel {
  readonly compact = input(false);
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

  protected readonly modeLabel = computed(() => MODE_LABELS[this.settings().mode]);
  protected readonly canFire = computed(() => this.turret.canFire() && this.fireForm().valid());

  protected fire(): void {
    const { mode, throttle, shots } = this.settings();
    this.turret.fire(mode, throttle, mode === 'burst' ? shots : 1);
  }

  protected nextMode(): void {
    this.settings.update((settings) => ({
      ...settings,
      mode: MODES[(MODES.indexOf(settings.mode) + 1) % MODES.length],
    }));
  }

  protected stepShots(delta: number): void {
    this.settings.update((settings) => ({
      ...settings,
      shots: Math.min(Math.max(settings.shots + delta, 1), MAX_BURST_SHOTS),
    }));
  }
}
