import { TestBed } from '@angular/core/testing';
import { App } from './app';
import { ARMED_READY_STATE, FakeTurretLink } from './link/fake-turret-link';
import { TURRET_LINK } from './link/turret-link';
import { Turret } from './turret/turret';

describe('App', () => {
  let link: FakeTurretLink;

  async function render() {
    TestBed.configureTestingModule({
      imports: [App],
      providers: [{ provide: TURRET_LINK, useValue: link }],
    });
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    return fixture;
  }

  const button = (host: HTMLElement, text: string) =>
    [...host.querySelectorAll('button')].find((b) => b.textContent?.trim() === text);

  beforeEach(() => {
    link = new FakeTurretLink();
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
});
