import { Component, computed, inject } from '@angular/core';
import { Turret } from '../turret/turret';
import { AimPad } from './aim-pad';
import { ArmSwitch } from './arm-switch';
import { FirePanel } from './fire-panel';
import { Telemetry } from './telemetry';
import { phaseText } from './turret-text';
import { VideoView } from './video-view';

@Component({
  selector: 'app-control-panel',
  imports: [AimPad, ArmSwitch, FirePanel, Telemetry, VideoView],
  template: `
    <header>
      <p class="arming" [class.armed]="turret.armed()">{{ turret.armed() ? 'Armed' : 'Safe' }}</p>
      @if (turret.state(); as state) {
        <dl>
          <div>
            <dt>Status</dt>
            <dd>{{ phase() }}</dd>
          </div>
          <div>
            <dt>Battery</dt>
            <dd>{{ battery() }}</dd>
          </div>
          <div>
            <dt>Aim</dt>
            <dd>{{ state.gimbal ? state.gimbal.pan + ' / ' + state.gimbal.tilt : 'Not moved' }}</dd>
          </div>
        </dl>
      }
      <button type="button" class="disconnect" (click)="turret.disconnect()">Disconnect</button>
    </header>

    <app-video-view class="video" />
    <app-arm-switch class="arm" />
    <app-aim-pad class="aim" />
    <app-fire-panel class="fire" />
    <app-telemetry class="telemetry" />
  `,
  styles: `
    :host {
      display: grid;
      flex: 1;
      grid-template-areas:
        'header'
        'video'
        'arm'
        'aim'
        'fire'
        'telemetry';
      gap: 12px;
      padding: 8px 16px 16px;
    }

    @media (orientation: landscape) and (min-width: 700px) {
      :host {
        grid-template-areas:
          'header header header'
          'video arm fire'
          'aim telemetry fire';
        grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr);
        grid-template-rows: auto auto minmax(0, 1fr);
        gap: 12px 20px;
        min-height: 0;
      }
    }

    header {
      grid-area: header;
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 8px 24px;
    }

    .arming {
      margin: 0;
      font-family: var(--font-label);
      font-size: 2rem;
      font-weight: 700;
      line-height: 1;
      color: var(--safe);
    }

    .arming.armed {
      color: var(--hazard);
    }

    dl {
      display: flex;
      flex-wrap: wrap;
      gap: 4px 20px;
      margin: 0;
    }

    dt {
      font-size: 0.8125rem;
      color: var(--ink-muted);
    }

    dd {
      margin: 0;
      font-family: var(--font-label);
      font-size: 1.25rem;
      font-weight: 500;
      font-variant-numeric: tabular-nums;
    }

    .disconnect {
      margin-left: auto;
      min-height: 40px;
      padding: 0 14px;
      border: 2px solid var(--panel-raised);
      border-radius: var(--radius-control);
      background: transparent;
    }

    .video {
      grid-area: video;
    }

    .aim {
      grid-area: aim;
    }

    .arm {
      grid-area: arm;
    }

    .telemetry {
      grid-area: telemetry;
      min-height: 0;
    }

    .fire {
      grid-area: fire;
    }
  `,
})
export class ControlPanel {
  protected readonly turret = inject(Turret);

  protected readonly phase = computed(() => {
    const state = this.turret.state();
    if (!state) return '';
    if (state.phase === 'idle' && !state.ready) return 'Cooling down';
    return phaseText(state.phase);
  });

  protected readonly battery = computed(() => {
    const volts = this.turret.batteryVolts();
    const percent = this.turret.state()?.batteryPct ?? 0;
    return volts === null ? 'No reading' : `${percent} %, ${volts.toFixed(1)} V`;
  });
}
