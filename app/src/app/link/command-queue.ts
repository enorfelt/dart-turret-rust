import { Command } from '../protocol/turret-protocol';

export class CommandQueue {
  private pending: Command[] = [];
  private draining = false;

  constructor(
    private readonly send: (command: Command) => Promise<void>,
    private readonly failed: (error: unknown, command: Command) => void,
  ) {}

  push(command: Command): void {
    switch (command.op) {
      case 'disarm':
        this.pending = [command];
        break;
      case 'cease-fire':
        this.pending = [command, ...this.pending.filter((queued) => queued.op !== 'fire')];
        break;
      case 'aim':
      case 'pusher': {
        const queued = this.pending.findIndex((pending) => pending.op === command.op);
        if (queued >= 0) {
          this.pending[queued] = command;
        } else {
          this.pending.push(command);
        }
        break;
      }
      default:
        this.pending.push(command);
    }
    void this.drain();
  }

  clear(): void {
    this.pending = [];
  }

  private async drain(): Promise<void> {
    if (this.draining) return;
    this.draining = true;
    for (let next = this.pending.shift(); next; next = this.pending.shift()) {
      try {
        await this.send(next);
      } catch (error) {
        this.failed(error, next);
      }
    }
    this.draining = false;
  }
}
