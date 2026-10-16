import type { VisionLink, VisionListener } from './vision-link';

export class FakeVisionLink implements VisionLink {
  opened: string[] = [];
  openError: unknown;
  closed = 0;
  private listener: VisionListener | undefined;

  open(url: string, listener: VisionListener): void {
    this.opened.push(url);
    if (this.openError) throw this.openError;
    this.listener = listener;
  }

  close(): void {
    this.closed++;
    this.listener = undefined;
  }

  send(bytes: readonly number[]): void {
    this.listener?.frame(new Uint8Array(bytes).buffer);
  }

  drop(): void {
    const listener = this.listener;
    this.listener = undefined;
    listener?.closed();
  }
}
