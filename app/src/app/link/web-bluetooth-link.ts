import {
  COMMAND_UUID,
  DEVICE_NAME_PREFIX,
  EVENT_UUID,
  SERVICE_UUID,
  STATE_UUID,
} from '../protocol/turret-protocol';
import type { LinkListener, TurretLink } from './turret-link';

export class WebBluetoothLink implements TurretLink {
  private device: BluetoothDevice | undefined;
  private command: BluetoothRemoteGATTCharacteristic | undefined;
  private releases: (() => void)[] = [];

  get supported(): boolean {
    return typeof navigator !== 'undefined' && 'bluetooth' in navigator;
  }

  async open(listener: LinkListener, reuseDevice: boolean): Promise<DataView> {
    this.release();
    const device =
      reuseDevice && this.device
        ? this.device
        : await navigator.bluetooth.requestDevice({
            filters: [{ namePrefix: DEVICE_NAME_PREFIX }],
            optionalServices: [SERVICE_UUID],
          });
    this.device = device;
    this.listen(device, 'gattserverdisconnected', () => {
      this.release();
      listener.disconnected();
    });

    try {
      const server = await device.gatt!.connect();
      const service = await server.getPrimaryService(SERVICE_UUID);
      const [command, state, event] = await Promise.all([
        service.getCharacteristic(COMMAND_UUID),
        service.getCharacteristic(STATE_UUID),
        service.getCharacteristic(EVENT_UUID),
      ]);
      this.listen(state, 'characteristicvaluechanged', () => listener.state(state.value!));
      this.listen(event, 'characteristicvaluechanged', () => listener.event(event.value!));
      await state.startNotifications();
      await event.startNotifications();
      const initialState = await state.readValue();
      this.command = command;
      return initialState;
    } catch (error) {
      this.close();
      throw error;
    }
  }

  async write(bytes: Uint8Array<ArrayBuffer>): Promise<void> {
    if (!this.command) {
      throw new DOMException('Turret is not connected', 'NetworkError');
    }
    await this.command.writeValueWithResponse(bytes);
  }

  close(): void {
    this.release();
    if (this.device?.gatt?.connected) {
      this.device.gatt.disconnect();
    }
  }

  private listen(target: EventTarget, type: string, handler: () => void): void {
    target.addEventListener(type, handler);
    this.releases.push(() => target.removeEventListener(type, handler));
  }

  private release(): void {
    this.releases.forEach((release) => release());
    this.releases = [];
    this.command = undefined;
  }
}
