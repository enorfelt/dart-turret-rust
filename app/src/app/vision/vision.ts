import { DestroyRef, Service, computed, inject, signal } from '@angular/core';
import { StreamError, VisionFrame, decodeVisionFrame } from '../protocol/vision-stream';
import { VISION_LINK, VisionListener } from './vision-link';

export type VisionConnection = 'idle' | 'connecting' | 'live' | 'lost';

export const DEFAULT_VISION_ADDRESS = '192.168.71.1';
export const STALE_AFTER_MS = 500;
export const RETRY_AFTER_MS = 1000;
const ADDRESS_KEY = 'dart-vision-address';
const ADDRESS_PATTERN = /^[A-Za-z0-9.-]+(:\d{1,5})?$/;

export function isValidVisionAddress(address: string): boolean {
  return ADDRESS_PATTERN.test(address);
}

export function isStale(receivedAt: number | null, now: number): boolean {
  return receivedAt === null || now - receivedAt > STALE_AFTER_MS;
}

function loadAddress(): string {
  try {
    const saved = localStorage.getItem(ADDRESS_KEY);
    return saved && isValidVisionAddress(saved) ? saved : DEFAULT_VISION_ADDRESS;
  } catch {
    return DEFAULT_VISION_ADDRESS;
  }
}

function saveAddress(address: string): void {
  try {
    localStorage.setItem(ADDRESS_KEY, address);
  } catch {
    // Remembering the address is a convenience only.
  }
}

@Service()
export class Vision {
  private readonly link = inject(VISION_LINK);
  private readonly listener: VisionListener = {
    frame: (data) => this.applyFrame(data),
    closed: () => this.linkClosed(),
  };
  private wanted = false;
  private retry: ReturnType<typeof setTimeout> | undefined;

  private readonly _address = signal(loadAddress());
  private readonly _connection = signal<VisionConnection>('idle');
  private readonly _frame = signal<VisionFrame | null>(null);
  private readonly _receivedAt = signal<number | null>(null);
  private readonly _error = signal<string | null>(null);

  readonly address = this._address.asReadonly();
  readonly connection = this._connection.asReadonly();
  readonly frame = this._frame.asReadonly();
  readonly receivedAt = this._receivedAt.asReadonly();
  readonly error = this._error.asReadonly();
  readonly active = computed(() => this.connection() !== 'idle');

  constructor() {
    inject(DestroyRef).onDestroy(() => this.disconnect());
  }

  connect(): void {
    this.wanted = true;
    this._error.set(null);
    this.open();
  }

  disconnect(): void {
    this.wanted = false;
    clearTimeout(this.retry);
    this.link.close();
    this._connection.set('idle');
    this._frame.set(null);
    this._receivedAt.set(null);
  }

  setAddress(address: string): boolean {
    const trimmed = address.trim();
    if (!isValidVisionAddress(trimmed)) return false;
    this._address.set(trimmed);
    saveAddress(trimmed);
    if (this.wanted) this.open();
    return true;
  }

  private open(): void {
    clearTimeout(this.retry);
    this._connection.set('connecting');
    try {
      this.link.open(`ws://${this.address()}/stream`, this.listener);
    } catch (error) {
      this._error.set(error instanceof Error ? error.message : String(error));
      this.linkClosed();
    }
  }

  private applyFrame(data: ArrayBuffer): void {
    try {
      this._frame.set(decodeVisionFrame(data));
    } catch (error) {
      if (!(error instanceof StreamError)) throw error;
      this.disconnect();
      this._error.set(error.message);
      return;
    }
    this._receivedAt.set(performance.now());
    this._connection.set('live');
    this._error.set(null);
  }

  private linkClosed(): void {
    if (!this.wanted) return;
    this._connection.set('lost');
    clearTimeout(this.retry);
    this.retry = setTimeout(() => this.open(), RETRY_AFTER_MS);
  }
}
