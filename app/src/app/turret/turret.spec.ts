import { DOCUMENT } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { ARMED_READY_STATE, FakeTurretLink, IDLE_STATE } from '../link/fake-turret-link';
import { TURRET_LINK } from '../link/turret-link';
import { Turret } from './turret';

const settle = () => new Promise((resolve) => setTimeout(resolve));

describe('Turret', () => {
  let link: FakeTurretLink;

  function create(): Turret {
    TestBed.configureTestingModule({ providers: [{ provide: TURRET_LINK, useValue: link }] });
    return TestBed.inject(Turret);
  }

  async function connected(): Promise<Turret> {
    const turret = create();
    await turret.connect();
    return turret;
  }

  beforeEach(() => {
    link = new FakeTurretLink();
  });

  it('reports unsupported browsers', () => {
    link.supported = false;

    expect(create().connection()).toBe('unsupported');
  });

  it('connects and exposes the first state', async () => {
    const turret = await connected();

    expect(turret.connection()).toBe('connected');
    expect(turret.state()).toEqual({
      armed: false,
      ready: true,
      phase: 'idle',
      batteryMv: 15_600,
      batteryPct: 71,
      gimbal: { pan: 512, tilt: 300 },
    });
    expect(turret.batteryVolts()).toBe(15.6);
  });

  it('refuses a turret with an unknown protocol version', async () => {
    link.initialState = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    const turret = await connected();

    expect(turret.connection()).toBe('idle');
    expect(turret.error()).toContain('version 2');
    expect(link.closed).toBe(1);
  });

  it('stays idle without an error when the chooser is cancelled', async () => {
    link.openError = new DOMException('User cancelled', 'NotFoundError');
    const turret = await connected();

    expect(turret.connection()).toBe('idle');
    expect(turret.error()).toBeNull();
  });

  it('follows state notifications', async () => {
    const turret = await connected();

    link.notifyState(ARMED_READY_STATE);

    expect(turret.armed()).toBe(true);
    expect(turret.canFire()).toBe(true);
  });

  it('cannot fire while armed but busy', async () => {
    const turret = await connected();

    link.notifyState([1, 0b01, 1, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]);

    expect(turret.armed()).toBe(true);
    expect(turret.canFire()).toBe(false);
  });

  it('logs events newest first and keeps the shot list', async () => {
    const turret = await connected();

    link.notifyEvent([0x01, 1, 0xfc, 0x00]);
    link.notifyEvent([0x01, 2, 0xff, 0xff]);
    link.notifyEvent([0x02, 2]);
    link.notifyEvent([0x09]);

    expect(turret.events().map((entry) => entry.event.kind)).toEqual(['finished', 'shot', 'shot']);
    expect(turret.shots()).toEqual([
      { number: 2, speedKmh: null },
      { number: 1, speedKmh: 252 },
    ]);
    expect(turret.lastShot()).toEqual({ number: 2, speedKmh: null });
  });

  it('encodes commands onto the link', async () => {
    const turret = await connected();

    turret.arm();
    await settle();
    turret.aim({ pan: 512, tilt: 300, panSpeed: 100, tiltSpeed: 50 });
    await settle();
    turret.fire('burst', 40, 3);
    await settle();
    turret.ceaseFire();
    await settle();
    turret.disarm();
    await settle();

    expect(link.written).toEqual([
      [0x01],
      [0x03, 0x00, 0x02, 0x2c, 0x01, 0x64, 0x00, 0x32, 0x00],
      [0x04, 0x01, 40, 3],
      [0x05],
      [0x02],
    ]);
  });

  it('sends nothing while disconnected', async () => {
    const turret = create();

    turret.arm();
    await settle();

    expect(link.written).toEqual([]);
  });

  it('disarms when the page is hidden while armed', async () => {
    const turret = await connected();
    link.notifyState(ARMED_READY_STATE);
    const document = TestBed.inject(DOCUMENT);
    vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');

    document.dispatchEvent(new Event('visibilitychange'));
    await settle();

    expect(turret.armed()).toBe(true);
    expect(link.written).toEqual([[0x02]]);
  });

  it('does not send disarm on hide when already safe', async () => {
    await connected();
    link.notifyState(IDLE_STATE);
    const document = TestBed.inject(DOCUMENT);
    vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');

    document.dispatchEvent(new Event('visibilitychange'));
    await settle();

    expect(link.written).toEqual([]);
  });

  it('marks the link lost and reconnects to the same device', async () => {
    const turret = await connected();

    link.drop();
    expect(turret.connection()).toBe('lost');
    expect(turret.state()).toBeNull();

    await turret.reconnect();
    expect(turret.connection()).toBe('connected');
    expect(link.opened).toEqual([{ reuseDevice: false }, { reuseDevice: true }]);
  });

  it('shows a failed write without dropping the link', async () => {
    const turret = await connected();
    link.write = () => Promise.reject(new DOMException('GATT Error: 0x81', 'NotSupportedError'));

    turret.arm();
    await settle();

    expect(turret.error()).toBe('The turret refused the command values.');
    expect(turret.connection()).toBe('connected');
  });
});
