import { DOCUMENT, InjectionToken, Service, effect, inject } from '@angular/core';
import { takeUntilDestroyed, toSignal } from '@angular/core/rxjs-interop';
import { SwUpdate } from '@angular/service-worker';
import { filter, fromEvent, interval, map, merge, takeUntil } from 'rxjs';
import { Turret } from '../turret/turret';

export const RELOAD = new InjectionToken<() => void>('RELOAD', {
  factory: () => {
    const location = inject(DOCUMENT).location;
    return () => location.reload();
  },
});

const CHECK_INTERVAL_MS = 15 * 60 * 1000;

@Service()
export class AppUpdate {
  private readonly swUpdate = inject(SwUpdate);
  private readonly turret = inject(Turret);
  private readonly reloadPage = inject(RELOAD);
  private reloading = false;

  private readonly versionReady = merge(
    this.swUpdate.versionUpdates.pipe(filter((event) => event.type === 'VERSION_READY')),
    this.swUpdate.unrecoverable,
  ).pipe(map(() => true));
  readonly ready = toSignal(this.versionReady, { initialValue: false });

  constructor() {
    if (!this.swUpdate.isEnabled) return;

    const document = inject(DOCUMENT);
    merge(fromEvent(document, 'visibilitychange'), interval(CHECK_INTERVAL_MS))
      .pipe(
        filter(() => document.visibilityState === 'visible'),
        takeUntil(this.versionReady),
        takeUntilDestroyed(),
      )
      .subscribe(() => this.swUpdate.checkForUpdate().catch(() => undefined));

    effect(() => {
      if (this.ready() && this.turret.inactive() && !this.turret.error()) void this.reload();
    });
  }

  async reload(): Promise<void> {
    if (this.reloading) return;
    this.reloading = true;
    try {
      await this.turret.disarmAndDisconnect();
      await this.swUpdate.activateUpdate();
    } finally {
      this.reloadPage();
    }
  }
}
