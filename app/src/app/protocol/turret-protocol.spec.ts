import {
  COMMAND_UUID,
  EVENT_UUID,
  ProtocolError,
  SERVICE_UUID,
  STATE_UUID,
  UnsupportedProtocolVersion,
  decodeEvent,
  decodeState,
  encodeCommand,
} from './turret-protocol';

const view = (...bytes: number[]) => new DataView(new Uint8Array(bytes).buffer);

describe('turret protocol', () => {
  it('derives characteristic UUIDs from the service UUID', () => {
    expect(SERVICE_UUID).toBe('8d6f0001-3b2a-4c1e-9f57-2e1d0c7a5b40');
    expect(COMMAND_UUID).toBe('8d6f0002-3b2a-4c1e-9f57-2e1d0c7a5b40');
    expect(STATE_UUID).toBe('8d6f0003-3b2a-4c1e-9f57-2e1d0c7a5b40');
    expect(EVENT_UUID).toBe('8d6f0004-3b2a-4c1e-9f57-2e1d0c7a5b40');
  });

  describe('encodeCommand', () => {
    it('encodes argument-free commands', () => {
      expect([...encodeCommand({ op: 'arm' })]).toEqual([0x01]);
      expect([...encodeCommand({ op: 'disarm' })]).toEqual([0x02]);
      expect([...encodeCommand({ op: 'cease-fire' })]).toEqual([0x05]);
    });

    it('encodes pusher hold and release', () => {
      expect([...encodeCommand({ op: 'pusher', pulseUs: 1200 })]).toEqual([0x06, 0xb0, 0x04]);
      expect([...encodeCommand({ op: 'pusher', pulseUs: 500 })]).toEqual([0x06, 0xf4, 0x01]);
      expect([...encodeCommand({ op: 'pusher', pulseUs: 2500 })]).toEqual([0x06, 0xc4, 0x09]);
      expect([...encodeCommand({ op: 'pusher', pulseUs: null })]).toEqual([0x06, 0x00, 0x00]);
    });

    it('encodes aim little-endian', () => {
      const bytes = encodeCommand({ op: 'aim', pan: 512, tilt: 300, panSpeed: 100, tiltSpeed: 50 });

      expect([...bytes]).toEqual([0x03, 0x00, 0x02, 0x2c, 0x01, 0x64, 0x00, 0x32, 0x00]);
    });

    it('encodes fire', () => {
      const bytes = encodeCommand({ op: 'fire', mode: 'burst', throttlePct: 40, shots: 3 });

      expect([...bytes]).toEqual([0x04, 0x01, 40, 3]);
    });

    it('encodes every fire mode', () => {
      const modeByte = (mode: 'single' | 'burst' | 'auto') =>
        encodeCommand({ op: 'fire', mode, throttlePct: 50, shots: 1 })[1];

      expect([modeByte('single'), modeByte('burst'), modeByte('auto')]).toEqual([0, 1, 2]);
    });
  });

  describe('decodeState', () => {
    it('decodes a full state', () => {
      expect(
        decodeState(view(3, 0b101, 2, 0xf0, 0x3c, 71, 0x00, 0x02, 0x2c, 0x01, 0xb0, 0x04)),
      ).toEqual({
        armed: true,
        ready: false,
        armSwitch: true,
        phase: 'pushing',
        batteryMv: 15_600,
        batteryPct: 71,
        gimbal: { pan: 512, tilt: 300 },
        heldPusherUs: 1200,
      });
    });

    it('decodes unknown values as null', () => {
      expect(decodeState(view(3, 0b010, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0, 0))).toEqual({
        armed: false,
        ready: true,
        armSwitch: false,
        phase: 'idle',
        batteryMv: null,
        batteryPct: 0,
        gimbal: null,
        heldPusherUs: null,
      });
    });

    it('refuses other protocol versions', () => {
      expect(() => decodeState(view(2, 0, 0, 0, 0, 0, 0, 0, 0, 0))).toThrow(
        UnsupportedProtocolVersion,
      );
    });

    it('rejects a short payload or an unknown phase', () => {
      expect(() => decodeState(view(3, 0, 0, 0, 0, 0, 0, 0, 0, 0))).toThrow(ProtocolError);
      expect(() => decodeState(view(3, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0))).toThrow(ProtocolError);
    });
  });

  describe('decodeEvent', () => {
    it('decodes shots with and without a speed reading', () => {
      expect(decodeEvent(view(0x01, 2, 0xfc, 0x00))).toEqual({
        kind: 'shot',
        number: 2,
        speedKmh: 252,
      });
      expect(decodeEvent(view(0x01, 2, 0xff, 0xff))).toEqual({
        kind: 'shot',
        number: 2,
        speedKmh: null,
      });
    });

    it('decodes the remaining events', () => {
      expect(decodeEvent(view(0x02, 3))).toEqual({ kind: 'finished', shots: 3 });
      expect(decodeEvent(view(0x03))).toEqual({ kind: 'aborted' });
      expect(decodeEvent(view(0x04, 2))).toEqual({ kind: 'fire-rejected', reason: 'cooling-down' });
      expect(decodeEvent(view(0x05))).toEqual({ kind: 'move-failed' });
    });

    it('decodes every rejection reason', () => {
      const reason = (code: number) => decodeEvent(view(0x04, code));

      expect([reason(0), reason(1), reason(2), reason(3)]).toEqual([
        { kind: 'fire-rejected', reason: 'disarmed' },
        { kind: 'fire-rejected', reason: 'busy' },
        { kind: 'fire-rejected', reason: 'cooling-down' },
        { kind: 'fire-rejected', reason: 'no-throttle' },
      ]);
    });

    it('ignores unknown or truncated events', () => {
      expect(decodeEvent(view(0x09))).toBeNull();
      expect(decodeEvent(view())).toBeNull();
      expect(decodeEvent(view(0x01, 2))).toBeNull();
      expect(decodeEvent(view(0x04, 7))).toBeNull();
    });
  });
});
