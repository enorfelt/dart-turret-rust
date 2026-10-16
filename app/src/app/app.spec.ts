import { TestBed } from '@angular/core/testing';
import { App } from './app';
import { ARMED_READY_STATE, FakeTurretLink } from './link/fake-turret-link';
import { TURRET_LINK } from './link/turret-link';
import { Turret } from './turret/turret';
import { SwUpdate } from '@angular/service-worker';
import { RELOAD } from './update/app-update';
import { APP_VERSION_TOKEN } from './update/app-version';
import { FakeSwUpdate } from './update/fake-sw-update';
import { FakeVisionLink } from './vision/fake-vision-link';
import { VISION_LINK } from './vision/vision-link';

describe('App', () => {
  let link: FakeTurretLink;
  let sw: FakeSwUpdate;
  let camera: FakeVisionLink;

  async function render() {
    TestBed.configureTestingModule({
      imports: [App],
      providers: [
        { provide: TURRET_LINK, useValue: link },
        { provide: SwUpdate, useValue: sw },
        { provide: VISION_LINK, useValue: camera },
        { provide: RELOAD, useValue: () => undefined },
        { provide: APP_VERSION_TOKEN, useValue: '2026-10-14-d7a9dbe' },
      ],
    });
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    return fixture;
  }

  const button = (host: HTMLElement, text: string) =>
    [...host.querySelectorAll('button')].find((b) => b.textContent?.trim() === text);

  beforeEach(() => {
    link = new FakeTurretLink();
    sw = new FakeSwUpdate();
    camera = new FakeVisionLink();
  });

  it('shows the app version on the connect panel', async () => {
    const fixture = await render();

    expect(fixture.nativeElement.textContent).toContain('Version 2026-10-14-d7a9dbe');
  });

  it('offers a reload when a new version is ready while connected', async () => {
    const fixture = await render();
    const host: HTMLElement = fixture.nativeElement;
    await TestBed.inject(Turret).connect();

    sw.versionReady();
    await fixture.whenStable();
    button(host, 'Reload')!.click();
    await fixture.whenStable();

    expect(host.textContent).toContain('A new version of the app is ready.');
    expect(sw.activations).toBe(1);
  });

  it('explains unsupported browsers', async () => {
    link.supported = false;
    const fixture = await render();

    expect(fixture.nativeElement.textContent).toContain('cannot use Bluetooth');
  });

  it('connects from the connect button and shows the controls', async () => {
    const fixture = await render();
    const host: HTMLElement = fixture.nativeElement;

    button(host, 'Connect')!.click();
    await fixture.whenStable();

    expect(host.textContent).toContain('Safe');
    expect(host.textContent).toContain('71 %, 15.6 V');
    expect(host.querySelector('app-joystick')).not.toBeNull();
  });

  it('does not offer arming while the arm switch is off', async () => {
    const fixture = await render();
    const host: HTMLElement = fixture.nativeElement;
    await TestBed.inject(Turret).connect();

    link.notifyState([2, 0b010, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]);
    await fixture.whenStable();

    expect(button(host, 'Arm switch off')!.disabled).toBe(true);
    expect(button(host, 'Hold to arm')).toBeUndefined();
  });

  it('wraps the screen in hazard tape and offers disarm while armed', async () => {
    const fixture = await render();
    const host: HTMLElement = fixture.nativeElement;
    await TestBed.inject(Turret).connect();

    link.notifyState(ARMED_READY_STATE);
    await fixture.whenStable();

    expect(host.classList).toContain('armed');
    button(host, 'Disarm')!.click();
    await fixture.whenStable();
    expect(link.written).toEqual([[0x02]]);
  });

  it('fires a burst with the chosen settings', async () => {
    const fixture = await render();
    const host: HTMLElement = fixture.nativeElement;
    await TestBed.inject(Turret).connect();
    link.notifyState(ARMED_READY_STATE);
    await fixture.whenStable();

    host.querySelector<HTMLInputElement>('input[value="burst"]')!.click();
    await fixture.whenStable();
    button(host, '+')!.click();
    await fixture.whenStable();
    button(host, 'Fire')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    await fixture.whenStable();

    expect(link.written).toEqual([[0x04, 0x01, 50, 4]]);
  });

  it('shows rejected fire attempts in the log', async () => {
    const fixture = await render();
    await TestBed.inject(Turret).connect();

    link.notifyEvent([0x04, 0]);
    await fixture.whenStable();

    expect(fixture.nativeElement.textContent).toContain('Did not fire: the turret is disarmed');
  });

  describe('with the camera on', () => {
    let fullscreenElement: Element | null;
    let requestFullscreen: ReturnType<typeof vi.fn>;
    let exitFullscreen: ReturnType<typeof vi.fn>;

    beforeEach(() => {
      localStorage.clear();
      fullscreenElement = null;
      requestFullscreen = vi.fn(async () => {
        fullscreenElement = document.documentElement;
      });
      exitFullscreen = vi.fn(async () => {
        fullscreenElement = null;
      });
      Object.defineProperty(document, 'fullscreenElement', {
        configurable: true,
        get: () => fullscreenElement,
      });
      Object.defineProperty(document, 'exitFullscreen', {
        configurable: true,
        value: exitFullscreen,
      });
      Object.defineProperty(document.documentElement, 'requestFullscreen', {
        configurable: true,
        value: requestFullscreen,
      });
    });

    afterEach(() => {
      delete (document as { fullscreenElement?: unknown }).fullscreenElement;
      delete (document as { exitFullscreen?: unknown }).exitFullscreen;
      delete (document.documentElement as { requestFullscreen?: unknown }).requestFullscreen;
    });

    async function showCamera() {
      const fixture = await render();
      const host: HTMLElement = fixture.nativeElement;
      await TestBed.inject(Turret).connect();
      await fixture.whenStable();
      button(host, 'Show camera')!.click();
      await fixture.whenStable();
      return { fixture, host };
    }

    it('fills the screen with the video and overlays the controls', async () => {
      const { fixture, host } = await showCamera();
      link.notifyState(ARMED_READY_STATE);
      await fixture.whenStable();

      expect(requestFullscreen).toHaveBeenCalled();
      expect(camera.opened).toHaveLength(1);
      expect(host.classList).toContain('camera');
      expect(host.querySelector('app-control-panel')!.classList).toContain('camera');
      expect(host.querySelector('canvas')).not.toBeNull();
      expect(host.querySelector('app-fire-panel')!.classList).toContain('compact');
      expect(getComputedStyle(host.querySelector('app-telemetry')!).display).toBe('none');
      expect(host.textContent).toContain('Last dart');
      expect(button(host, 'Disconnect')).toBeUndefined();
      expect(button(host, 'Disarm')).toBeDefined();
      expect(button(host, 'Fire')).toBeDefined();
      expect(host.querySelector('app-joystick')).not.toBeNull();
    });

    it('fires a burst after cycling the mode from one button', async () => {
      const { fixture, host } = await showCamera();
      link.notifyState(ARMED_READY_STATE);
      await fixture.whenStable();
      const mode = host.querySelector<HTMLButtonElement>('button.cycle')!;

      mode.click();
      await fixture.whenStable();
      expect(mode.textContent).toContain('Burst');
      button(host, 'Fire')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
      await fixture.whenStable();

      expect(link.written).toEqual([[0x04, 0x01, 50, 3]]);
    });

    it('returns to the normal layout and leaves full screen when the camera is hidden', async () => {
      const { fixture, host } = await showCamera();

      button(host, 'Hide camera')!.click();
      await fixture.whenStable();

      expect(exitFullscreen).toHaveBeenCalled();
      expect(host.classList).not.toContain('camera');
      expect(host.querySelector('canvas')).toBeNull();
      expect(button(host, 'Disconnect')).toBeDefined();
      expect(button(host, 'Show camera')).toBeDefined();
    });
  });
});
