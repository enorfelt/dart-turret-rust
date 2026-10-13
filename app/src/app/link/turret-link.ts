import { InjectionToken } from '@angular/core';
import { WebBluetoothLink } from './web-bluetooth-link';

export interface LinkListener {
  state(view: DataView): void;
  event(view: DataView): void;
  disconnected(): void;
}

export interface TurretLink {
  readonly supported: boolean;
  open(listener: LinkListener, reuseDevice: boolean): Promise<DataView>;
  write(bytes: Uint8Array<ArrayBuffer>): Promise<void>;
  close(): void;
}

export const TURRET_LINK = new InjectionToken<TurretLink>('TurretLink', {
  factory: () => new WebBluetoothLink(),
});
