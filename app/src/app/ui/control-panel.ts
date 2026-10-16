import { Component, DestroyRef, computed, effect, inject } from '@angular/core';
import { Turret } from '../turret/turret';
import { Vision } from '../vision/vision';
import { AimPad } from './aim-pad';
import { ArmSwitch } from './arm-switch';
import { FirePanel } from './fire-panel';
import { exitFullscreen } from './fullscreen';
import { Telemetry } from './telemetry';
import { phaseText } from './turret-text';
import { VideoView } from './video-view';

@Component({
  selector: 'app-control-panel',
  imports: [AimPad, ArmSwitch, FirePanel, Telemetry, VideoView],
  host: { '[class.camera]': 'vision.active()' },
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
          @if (vision.active()) {
            <div>
              <dt>Last dart</dt>
              <dd>{{ lastDart() }}</dd>
            </div>
          } @else {
            <div>
              <dt>Aim</dt>
              <dd>
                {{ state.gimbal ? state.gimbal.pan + ' / ' + state.gimbal.tilt : 'Not moved' }}
              </dd>
            </div>
          }
        </dl>
      }
      @if (vision.active()) {
        <button type="button" class="end" (click)="vision.disconnect()">Hide camera</button>
      } @else {
        <button type="button" class="end" (click)="turret.disconnect()">Disconnect</button>
      }
    </header>

    <app-video-view class="video" />
    <app-arm-switch class="arm" />
    <app-aim-pad class="aim" />
    <app-fire-panel class="fire" [compact]="vision.active()" />
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

    .end {
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

    :host(.camera) {
      position: relative;
      grid-template-areas:
        'header header'
        '. arm'
        '. .'
        'aim fire';
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      grid-template-rows: auto auto minmax(0, 1fr) auto;
      gap: 8px;
      min-height: 0;
      padding: 8px;
    }

    @media (orientation: landscape) {
      :host(.camera) {
        grid-template-areas:
          'header header arm'
          '. . .'
          'aim . fire';
        grid-template-columns: auto minmax(0, 1fr) min(34vw, 260px);
        grid-template-rows: auto minmax(0, 1fr) auto;
      }
    }

    :host(.camera) .video {
      grid-area: 1 / 1 / -1 / -1;
      z-index: 0;
      margin: -8px;
    }

    :host(.camera) :is(header, .arm, .aim, .fire) {
      z-index: 1;
    }

    :host(.camera) :is(header, .arm, .fire) {
      border-radius: var(--radius-control);
      background: rgb(0 0 0 / 70%);
    }

    :host(.camera) :is(header, .fire) {
      padding: 6px 12px;
    }

    :host(.camera) header {
      align-self: start;
      gap: 4px 16px;
    }

    :host(.camera) .arming {
      font-size: 1.5rem;
    }

    :host(.camera) dt {
      color: var(--ink);
    }

    :host(.camera) dd {
      font-size: 1.125rem;
    }

    :host(.camera) .arm {
      align-self: start;
    }

    :host(.camera) .aim {
      align-self: end;
      justify-self: start;
      width: min(38vw, 42dvh, 220px);
    }

    :host(.camera) .fire {
      align-self: end;
    }

    :host(.camera) .telemetry {
      display: none;
    }
  `,
})
export class ControlPanel {
  protected readonly turret = inject(Turret);
  protected readonly vision = inject(Vision);

  constructor() {
    effect(() => {
      if (!this.vision.active()) void exitFullscreen();
    });
    inject(DestroyRef).onDestroy(() => void exitFullscreen());
  }

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

  protected readonly lastDart = computed(() => {
    const shot = this.turret.lastShot();
    if (!shot) return 'None yet';
    return shot.speedKmh === null ? 'No speed' : `${shot.speedKmh} km/h`;
  });
}
