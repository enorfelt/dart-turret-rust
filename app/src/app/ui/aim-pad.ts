import { Component, DestroyRef, inject } from '@angular/core';
import { AimController, Deflection } from '../aim/aim-controller';
import { Turret } from '../turret/turret';
import { Joystick } from './joystick';

@Component({
  selector: 'app-aim-pad',
  imports: [Joystick],
  template: `
    <app-joystick (engaged)="start()" (deflected)="deflection = $event" (released)="stop()" />
  `,
  styles: `
    :host {
      display: grid;
      place-items: center;
    }
  `,
})
export class AimPad {
  private readonly turret = inject(Turret);
  private readonly aim = new AimController();
  protected deflection: Deflection = { x: 0, y: 0 };
  private frame: number | null = null;

  constructor() {
    inject(DestroyRef).onDestroy(() => this.stop());
  }

  protected start(): void {
    this.aim.begin(this.turret.state()?.gimbal ?? null, performance.now());
    this.frame = requestAnimationFrame(this.tick);
  }

  protected stop(): void {
    if (this.frame === null) return;
    cancelAnimationFrame(this.frame);
    this.frame = null;
    const last = this.aim.end();
    if (last) this.turret.aim(last);
  }

  private readonly tick = (now: number): void => {
    const command = this.aim.step(this.deflection, now);
    if (command) this.turret.aim(command);
    this.frame = requestAnimationFrame(this.tick);
  };
}
