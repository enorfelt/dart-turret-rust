import type { LinkListener, TurretLink } from './turret-link';

const view = (bytes: readonly number[]) => new DataView(new Uint8Array(bytes).buffer);

export const IDLE_STATE = [1, 0b10, 0, 0xf0, 0x3c, 71, 0x00, 0x02, 0x2c, 0x01] as const;
export const ARMED_READY_STATE = [1, 0b11, 0, 0xf0, 0x3c, 71, 0x00, 0x02, 0x2c, 0x01] as const;

export class FakeTurretLink implements TurretLink {
  supported = true;
  initialState: readonly number[] = IDLE_STATE;
  openError: unknown;
  written: number[][] = [];
  opened: { reuseDevice: boolean }[] = [];
  closed = 0;
  private listener: LinkListener | undefined;

  async open(listener: LinkListener, reuseDevice: boolean): Promise<DataView> {
    this.opened.push({ reuseDevice });
    if (this.openError) throw this.openError;
    this.listener = listener;
    return view(this.initialState);
  }

  async write(bytes: Uint8Array<ArrayBuffer>): Promise<void> {
    this.written.push([...bytes]);
  }

  close(): void {
    this.closed++;
    this.listener = undefined;
  }

  notifyState(bytes: readonly number[]): void {
    this.listener?.state(view(bytes));
  }

  notifyEvent(bytes: readonly number[]): void {
    this.listener?.event(view(bytes));
  }

  drop(): void {
    const listener = this.listener;
    this.listener = undefined;
    listener?.disconnected();
  }
}
