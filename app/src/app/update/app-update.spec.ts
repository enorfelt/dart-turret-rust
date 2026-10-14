import { TestBed } from '@angular/core/testing';
import { SwUpdate } from '@angular/service-worker';
import { ARMED_READY_STATE, FakeTurretLink } from '../link/fake-turret-link';
import { TURRET_LINK } from '../link/turret-link';
import { Turret } from '../turret/turret';
import { AppUpdate, RELOAD } from './app-update';
import { FakeSwUpdate } from './fake-sw-update';

describe('AppUpdate', () => {
  let link: FakeTurretLink;
  let sw: FakeSwUpdate;
  let reloads: number;

  function create() {
    TestBed.configureTestingModule({
      providers: [
        { provide: TURRET_LINK, useValue: link },
        { provide: SwUpdate, useValue: sw },
        { provide: RELOAD, useValue: () => reloads++ },
      ],
    });
    return TestBed.inject(AppUpdate);
  }

  async function settle() {
    TestBed.tick();
    await new Promise((resolve) => setTimeout(resolve));
  }

  beforeEach(() => {
    link = new FakeTurretLink();
    sw = new FakeSwUpdate();
    reloads = 0;
  });

  it('reloads into a new version while disconnected', async () => {
    create();

    sw.versionReady();
    await settle();

    expect(sw.activations).toBe(1);
    expect(reloads).toBe(1);
  });

  it('asks instead of reloading while connected, then reloads once disconnected', async () => {
    const update = create();
    const turret = TestBed.inject(Turret);
    await turret.connect();

    sw.versionReady();
    await settle();

    expect(update.ready()).toBe(true);
    expect(reloads).toBe(0);

    turret.disconnect();
    await settle();

    expect(reloads).toBe(1);
  });

  it('keeps the reconnect option when the link drops with an update pending', async () => {
    const update = create();
    await TestBed.inject(Turret).connect();
    sw.versionReady();

    link.drop();
    await settle();

    expect(update.ready()).toBe(true);
    expect(reloads).toBe(0);
  });

  it('waits for a connect error to be dismissed before reloading', async () => {
    create();
    const turret = TestBed.inject(Turret);
    link.openError = new DOMException('Authentication failed', 'SecurityError');
    sw.versionReady();
    await turret.connect();
    await settle();

    expect(turret.error()).not.toBeNull();
    expect(reloads).toBe(0);

    turret.dismissError();
    await settle();

    expect(reloads).toBe(1);
  });

  it('reloads after an unrecoverable service worker state once disconnected', async () => {
    create();

    sw.unrecoverable.next({ type: 'UNRECOVERABLE_STATE', reason: 'missing chunk' });
    await settle();

    expect(reloads).toBe(1);
  });

  it('disarms and disconnects before reloading from the banner', async () => {
    const update = create();
    const turret = TestBed.inject(Turret);
    await turret.connect();
    link.notifyState(ARMED_READY_STATE);
    sw.versionReady();

    await update.reload();

    expect(link.written).toEqual([[0x02]]);
    expect(link.closed).toBe(1);
    expect(reloads).toBe(1);
  });

  it('checks for a new version when the app becomes visible, until one is ready', () => {
    create();

    document.dispatchEvent(new Event('visibilitychange'));
    sw.versionReady();
    document.dispatchEvent(new Event('visibilitychange'));

    expect(sw.checks).toBe(1);
  });

  it('never checks without a service worker', () => {
    sw.isEnabled = false;
    create();

    document.dispatchEvent(new Event('visibilitychange'));

    expect(sw.checks).toBe(0);
  });
});
