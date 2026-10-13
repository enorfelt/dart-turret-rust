import { Component, inject } from '@angular/core';
import { Turret } from '../turret/turret';

@Component({
  selector: 'app-connect-panel',
  template: `
    <h1>DART Turret</h1>
    @switch (turret.connection()) {
      @case ('unsupported') {
        <p>
          This browser cannot use Bluetooth. Open the app in Chrome or Edge on Android or a desktop
          computer. Safari and Firefox do not support Web Bluetooth.
        </p>
      }
      @case ('lost') {
        <p>The connection dropped and the turret disarmed itself.</p>
        <button type="button" class="primary" (click)="turret.reconnect()">Reconnect</button>
        <button type="button" class="secondary" (click)="turret.connect()">
          Choose another turret
        </button>
      }
      @case ('connecting') {
        <p>Connecting. On first use, enter the turret's six-digit passkey when asked.</p>
        <button type="button" class="primary" disabled>Connecting…</button>
      }
      @default {
        <p>Switch the turret on and keep the phone within a few metres of it.</p>
        <button type="button" class="primary" (click)="turret.connect()">Connect</button>
      }
    }
  `,
  styles: `
    :host {
      display: flex;
      flex: 1;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 16px;
      padding: 24px 16px;
      text-align: center;
    }

    h1 {
      margin: 0;
      font-family: var(--font-label);
      font-size: clamp(3rem, 12vw, 5.5rem);
      font-weight: 700;
      line-height: 0.9;
      letter-spacing: 0.01em;
    }

    p {
      max-width: 34ch;
      margin: 0;
      color: var(--ink-muted);
    }

    button {
      min-width: 220px;
      min-height: 52px;
      padding: 0 24px;
      border-radius: var(--radius-control);
      font-family: var(--font-label);
      font-size: 1.375rem;
      font-weight: 700;
    }

    .primary {
      border: 0;
      background: var(--safe);
      color: var(--body);
    }

    .primary:disabled {
      opacity: 0.6;
    }

    .secondary {
      border: 2px solid var(--panel-raised);
      background: transparent;
    }
  `,
})
export class ConnectPanel {
  protected readonly turret = inject(Turret);
}
