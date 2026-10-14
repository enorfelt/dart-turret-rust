import { SwUpdate, UnrecoverableStateEvent, VersionEvent } from '@angular/service-worker';
import { Subject } from 'rxjs';

export class FakeSwUpdate implements Pick<
  SwUpdate,
  'isEnabled' | 'versionUpdates' | 'unrecoverable' | 'checkForUpdate' | 'activateUpdate'
> {
  isEnabled = true;
  readonly versionUpdates = new Subject<VersionEvent>();
  readonly unrecoverable = new Subject<UnrecoverableStateEvent>();
  checks = 0;
  activations = 0;

  async checkForUpdate(): Promise<boolean> {
    this.checks++;
    return false;
  }

  async activateUpdate(): Promise<boolean> {
    this.activations++;
    return true;
  }

  versionReady(): void {
    const version = { hash: 'new' };
    this.versionUpdates.next({
      type: 'VERSION_READY',
      currentVersion: version,
      latestVersion: version,
    });
  }
}
