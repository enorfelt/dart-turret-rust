const characteristicUuid = (id: string) => `8d6f${id}-3b2a-4c1e-9f57-2e1d0c7a5b40`;

export const SERVICE_UUID = characteristicUuid('0001');
export const COMMAND_UUID = characteristicUuid('0002');
export const STATE_UUID = characteristicUuid('0003');
export const EVENT_UUID = characteristicUuid('0004');
export const DEVICE_NAME_PREFIX = 'DART';

export const PROTOCOL_VERSION = 3;
export const PUSHER_MIN_US = 500;
export const PUSHER_MAX_US = 2500;
const STATE_LENGTH = 12;
const UNKNOWN = 0xffff;

export type FireMode = 'single' | 'burst' | 'auto';
export type FirePhase = 'idle' | 'spin-up' | 'pushing' | 'retracting' | 'spin-down';
export type Rejection = 'disarmed' | 'busy' | 'cooling-down' | 'no-throttle';

const FIRE_MODES: readonly FireMode[] = ['single', 'burst', 'auto'];
const PHASES: readonly FirePhase[] = ['idle', 'spin-up', 'pushing', 'retracting', 'spin-down'];
const REJECTIONS: readonly Rejection[] = ['disarmed', 'busy', 'cooling-down', 'no-throttle'];

export type Command =
  | { op: 'arm' }
  | { op: 'disarm' }
  | { op: 'aim'; pan: number; tilt: number; panSpeed: number; tiltSpeed: number }
  | { op: 'fire'; mode: FireMode; throttlePct: number; shots: number }
  | { op: 'cease-fire' }
  | { op: 'pusher'; pulseUs: number | null };

export type AimCommand = Omit<Extract<Command, { op: 'aim' }>, 'op'>;

export interface GimbalPosition {
  pan: number;
  tilt: number;
}

export interface TurretState {
  armed: boolean;
  ready: boolean;
  armSwitch: boolean;
  phase: FirePhase;
  batteryMv: number | null;
  batteryPct: number;
  gimbal: GimbalPosition | null;
  heldPusherUs: number | null;
}

export type TurretEvent =
  | { kind: 'shot'; number: number; speedKmh: number | null }
  | { kind: 'finished'; shots: number }
  | { kind: 'aborted' }
  | { kind: 'fire-rejected'; reason: Rejection }
  | { kind: 'move-failed' };

export class ProtocolError extends Error {
  override name = 'ProtocolError';
}

export class UnsupportedProtocolVersion extends ProtocolError {
  override name = 'UnsupportedProtocolVersion';

  constructor(readonly version: number) {
    super(`Turret speaks protocol version ${version}, this app only knows ${PROTOCOL_VERSION}`);
  }
}

export function encodeCommand(command: Command): Uint8Array<ArrayBuffer> {
  switch (command.op) {
    case 'arm':
      return Uint8Array.of(0x01);
    case 'disarm':
      return Uint8Array.of(0x02);
    case 'aim': {
      const bytes = new Uint8Array(9);
      const view = new DataView(bytes.buffer);
      view.setUint8(0, 0x03);
      view.setUint16(1, command.pan, true);
      view.setUint16(3, command.tilt, true);
      view.setUint16(5, command.panSpeed, true);
      view.setUint16(7, command.tiltSpeed, true);
      return bytes;
    }
    case 'fire':
      return Uint8Array.of(
        0x04,
        FIRE_MODES.indexOf(command.mode),
        command.throttlePct,
        command.shots,
      );
    case 'cease-fire':
      return Uint8Array.of(0x05);
    case 'pusher': {
      const bytes = new Uint8Array(3);
      const view = new DataView(bytes.buffer);
      view.setUint8(0, 0x06);
      view.setUint16(1, command.pulseUs ?? 0, true);
      return bytes;
    }
  }
}

export function decodeState(view: DataView): TurretState {
  if (view.byteLength < 1) {
    throw new ProtocolError('Empty state payload');
  }
  const version = view.getUint8(0);
  if (version !== PROTOCOL_VERSION) {
    throw new UnsupportedProtocolVersion(version);
  }
  if (view.byteLength < STATE_LENGTH) {
    throw new ProtocolError(`State payload is ${view.byteLength} bytes, expected ${STATE_LENGTH}`);
  }
  const flags = view.getUint8(1);
  const phase = PHASES[view.getUint8(2)];
  if (phase === undefined) {
    throw new ProtocolError(`Unknown fire phase ${view.getUint8(2)}`);
  }
  const batteryMv = view.getUint16(3, true);
  const pan = view.getUint16(6, true);
  const tilt = view.getUint16(8, true);
  const heldPusherUs = view.getUint16(10, true);
  return {
    armed: (flags & 0b001) !== 0,
    ready: (flags & 0b010) !== 0,
    armSwitch: (flags & 0b100) !== 0,
    phase,
    batteryMv: batteryMv === 0 ? null : batteryMv,
    batteryPct: view.getUint8(5),
    gimbal: pan === UNKNOWN || tilt === UNKNOWN ? null : { pan, tilt },
    heldPusherUs: heldPusherUs === 0 ? null : heldPusherUs,
  };
}

export function decodeEvent(view: DataView): TurretEvent | null {
  const length = view.byteLength;
  const kind = length > 0 ? view.getUint8(0) : undefined;
  switch (kind) {
    case 0x01: {
      if (length < 4) return null;
      const speed = view.getUint16(2, true);
      return { kind: 'shot', number: view.getUint8(1), speedKmh: speed === UNKNOWN ? null : speed };
    }
    case 0x02:
      return length < 2 ? null : { kind: 'finished', shots: view.getUint8(1) };
    case 0x03:
      return { kind: 'aborted' };
    case 0x04: {
      const reason = length < 2 ? undefined : REJECTIONS[view.getUint8(1)];
      return reason === undefined ? null : { kind: 'fire-rejected', reason };
    }
    case 0x05:
      return { kind: 'move-failed' };
    default:
      return null;
  }
}
