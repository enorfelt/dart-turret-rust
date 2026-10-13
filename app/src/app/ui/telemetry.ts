import { Component, computed, inject } from '@angular/core';
import { Turret } from '../turret/turret';
import { eventText } from './turret-text';

@Component({
  selector: 'app-telemetry',
  template: `
    <section class="last-shot" aria-live="polite">
      <h2>Last dart</h2>
      @if (turret.lastShot(); as shot) {
        <p class="speed">
          @if (shot.speedKmh === null) {
            <span class="value">—</span>
            <span class="unit">no speed reading</span>
          } @else {
            <span class="value">{{ shot.speedKmh }}</span>
            <span class="unit">km/h</span>
          }
        </p>
        <p class="summary">{{ shotSummary() }}</p>
      } @else {
        <p class="summary">Arm the turret and fire to see dart speeds here.</p>
      }
    </section>

    <section class="log">
      <h2>Turret log</h2>
      <ol>
        @for (entry of turret.events(); track entry.id) {
          <li
            [class.warning]="
              entry.event.kind === 'fire-rejected' || entry.event.kind === 'move-failed'
            "
          >
            {{ text(entry.event) }}
          </li>
        } @empty {
          <li class="empty">Nothing yet.</li>
        }
      </ol>
    </section>
  `,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      gap: 16px;
      min-height: 0;
    }

    h2 {
      margin: 0 0 4px;
      font-family: var(--font-label);
      font-size: 1.125rem;
      font-weight: 500;
      color: var(--ink-muted);
    }

    p {
      margin: 0;
    }

    .speed {
      display: flex;
      align-items: baseline;
      gap: 8px;
      font-family: var(--font-label);
    }

    .value {
      font-size: 4.5rem;
      font-weight: 700;
      line-height: 0.9;
      font-variant-numeric: tabular-nums;
    }

    .unit {
      font-size: 1.375rem;
      color: var(--ink-muted);
    }

    .summary {
      color: var(--ink-muted);
    }

    .log {
      flex: 1;
      min-height: 0;
      overflow-y: auto;
    }

    ol {
      margin: 0;
      padding: 0;
      list-style: none;
    }

    li {
      padding: 6px 0;
      border-bottom: 1px solid var(--panel-raised);
    }

    li.warning {
      color: var(--hazard);
    }

    li.empty {
      color: var(--ink-muted);
      border-bottom: 0;
    }
  `,
})
export class Telemetry {
  protected readonly turret = inject(Turret);
  protected readonly text = eventText;

  protected readonly shotSummary = computed(() => {
    const speeds = this.turret
      .shots()
      .map((shot) => shot.speedKmh)
      .filter((speed) => speed !== null);
    const count = this.turret.shots().length;
    if (speeds.length === 0) return `${count} ${count === 1 ? 'dart' : 'darts'} this session`;
    const average = Math.round(speeds.reduce((sum, speed) => sum + speed, 0) / speeds.length);
    return `${count} ${count === 1 ? 'dart' : 'darts'} this session, average ${average} km/h`;
  });
}
