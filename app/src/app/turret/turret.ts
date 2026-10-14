import { DOCUMENT, DestroyRef, Service, computed, inject, signal } from '@angular/core';
import { CommandQueue } from '../link/command-queue';
import { describeLinkError } from '../link/link-error';
import { LinkListener, TURRET_LINK } from '../link/turret-link';
import {
  AimCommand,
  Command,
  FireMode,
  TurretEvent,
  TurretState,
  decodeEvent,
  decodeState,
  encodeCommand,
} from '../protocol/turret-protocol';

export type Connection = 'unsupported' | 'idle' | 'connecting' | 'connected' | 'lost';

export interface LoggedEvent {
  id: number;
  event: TurretEvent;
}

export interface Shot {
  number: number;
  speedKmh: number | null;
}

export interface ShotTally {
  shots: number;
  timedShots: number;
  speedSumKmh: number;
}

const EVENT_LOG_SIZE = 50;
const SHOT_LOG_SIZE = 100;

@Service()
export class Turret {
  private readonly link = inject(TURRET_LINK);
  private readonly queue = new CommandQueue(
    async (command) => this.link.write(encodeCommand(command)),
    (error) => this._error.set(describeLinkError(error)),
  );
  private readonly listener: LinkListener = {
    state: (view) => this.applyState(view),
    event: (view) => this.applyEvent(view),
    disconnected: () => this.linkLost(),
  };
  private nextEventId = 0;

  private readonly _connection = signal<Connection>(this.link.supported ? 'idle' : 'unsupported');
  private readonly _state = signal<TurretState | null>(null);
  private readonly _events = signal<readonly LoggedEvent[]>([]);
  private readonly _shots = signal<readonly Shot[]>([]);
  private readonly _tally = signal<ShotTally>({ shots: 0, timedShots: 0, speedSumKmh: 0 });
  private readonly _error = signal<string | null>(null);

  readonly connection = this._connection.asReadonly();
  readonly state = this._state.asReadonly();
  readonly events = this._events.asReadonly();
  readonly shots = this._shots.asReadonly();
  readonly tally = this._tally.asReadonly();
  readonly error = this._error.asReadonly();

  readonly connected = computed(() => this.connection() === 'connected');
  readonly inactive = computed(() => ['idle', 'unsupported'].includes(this.connection()));
  readonly armed = computed(() => this.connected() && this.state()?.armed === true);
  readonly canFire = computed(() => this.armed() && this.state()?.ready === true);
  readonly lastShot = computed(() => this.shots()[0] ?? null);
  readonly batteryVolts = computed(() => {
    const millivolts = this.state()?.batteryMv;
    return millivolts == null ? null : millivolts / 1000;
  });

  constructor() {
    const document = inject(DOCUMENT);
    const disarmWhenHidden = () => {
      if (document.visibilityState === 'hidden' && this.armed()) {
        this.queue.push({ op: 'disarm' });
      }
    };
    document.addEventListener('visibilitychange', disarmWhenHidden);
    inject(DestroyRef).onDestroy(() =>
      document.removeEventListener('visibilitychange', disarmWhenHidden),
    );
  }

  connect(): Promise<void> {
    return this.open(false);
  }

  reconnect(): Promise<void> {
    return this.open(true);
  }

  disconnect(): void {
    this.queue.clear();
    this.link.close();
    this._state.set(null);
    this._connection.set('idle');
  }

  async disarmAndDisconnect(): Promise<void> {
    if (this.connected()) {
      this.queue.clear();
      await this.link.write(encodeCommand({ op: 'disarm' })).catch(() => undefined);
    }
    this.disconnect();
  }

  dismissError(): void {
    this._error.set(null);
  }

  arm(): void {
    this.send({ op: 'arm' });
  }

  disarm(): void {
    this.send({ op: 'disarm' });
  }

  aim(aim: AimCommand): void {
    this.send({ op: 'aim', ...aim });
  }

  fire(mode: FireMode, throttlePct: number, shots: number): void {
    this.send({ op: 'fire', mode, throttlePct, shots });
  }

  ceaseFire(): void {
    this.send({ op: 'cease-fire' });
  }

  private send(command: Command): void {
    if (this.connected()) {
      this.queue.push(command);
    }
  }

  private async open(reuseDevice: boolean): Promise<void> {
    this._connection.set('connecting');
    this._state.set(null);
    this._error.set(null);
    try {
      const initialState = await this.link.open(this.listener, reuseDevice);
      if (this._connection() !== 'connecting') return;
      // A notification that arrived while opening is at least as fresh as the read.
      this._state.update((state) => state ?? decodeState(initialState));
      this._connection.set('connected');
    } catch (error) {
      this.link.close();
      this._connection.set(reuseDevice ? 'lost' : 'idle');
      this._error.set(describeLinkError(error));
    }
  }

  private applyState(view: DataView): void {
    try {
      this._state.set(decodeState(view));
    } catch (error) {
      this._error.set(describeLinkError(error));
      this.disconnect();
    }
  }

  private applyEvent(view: DataView): void {
    const event = decodeEvent(view);
    if (!event) return;
    this._events.update((events) =>
      [{ id: this.nextEventId++, event }, ...events].slice(0, EVENT_LOG_SIZE),
    );
    if (event.kind === 'shot') {
      const shot = { number: event.number, speedKmh: event.speedKmh };
      this._shots.update((shots) => [shot, ...shots].slice(0, SHOT_LOG_SIZE));
      this._tally.update(({ shots, timedShots, speedSumKmh }) => ({
        shots: shots + 1,
        timedShots: timedShots + (shot.speedKmh === null ? 0 : 1),
        speedSumKmh: speedSumKmh + (shot.speedKmh ?? 0),
      }));
    }
  }

  private linkLost(): void {
    this.queue.clear();
    this._state.set(null);
    this._connection.set('lost');
  }
}
