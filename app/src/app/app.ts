import { Component, inject } from '@angular/core';
import { Turret } from './turret/turret';
import { AppUpdate } from './update/app-update';
import { ConnectPanel } from './ui/connect-panel';
import { ControlPanel } from './ui/control-panel';

@Component({
  selector: 'app-root',
  imports: [ConnectPanel, ControlPanel],
  host: { '[class.armed]': 'turret.armed()' },
  template: `
    @if (update.ready()) {
      <p class="banner update" role="status">
        <span>A new version of the app is ready.</span>
        <button type="button" (click)="update.reload()">Reload</button>
      </p>
    }
    @if (turret.error(); as error) {
      <p class="banner error" role="alert">
        <span>{{ error }}</span>
        <button type="button" (click)="turret.dismissError()">Dismiss</button>
      </p>
    }
    @let connection = turret.connection();
    @switch (connection) {
      @case ('connected') {
        <app-control-panel />
      }
      @case ('unsupported') {
        <app-connect-panel />
      }
      @case ('idle') {
        <app-connect-panel />
      }
      @case ('connecting') {
        <app-connect-panel />
      }
      @case ('lost') {
        <app-connect-panel />
      }
      @default never;
    }
  `,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      min-height: 100dvh;
      border: 8px solid var(--panel);
    }

    @media (orientation: landscape) and (min-width: 700px) {
      :host {
        height: 100dvh;
      }
    }

    :host(.armed) {
      border-image: repeating-linear-gradient(-45deg, var(--hazard) 0 14px, var(--body) 14px 28px) 8;
    }

    .banner {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: var(--gap);
      margin: 0;
      padding: 8px 12px;
    }

    .update {
      background: var(--panel-raised);
    }

    .error {
      background: var(--fire-pressed);
      color: #fff;
    }

    .banner button {
      flex: none;
      padding: 4px 12px;
      border: 1px solid currentColor;
      border-radius: var(--radius-control);
      background: transparent;
    }
  `,
})
export class App {
  protected readonly turret = inject(Turret);
  protected readonly update = inject(AppUpdate);
}
