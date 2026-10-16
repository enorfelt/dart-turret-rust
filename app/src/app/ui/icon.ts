import { Component, input } from '@angular/core';

export type IconName = 'power' | 'target';

@Component({
  selector: 'app-icon',
  host: { 'aria-hidden': 'true' },
  template: `
    @switch (name()) {
      @case ('power') {
        <svg viewBox="0 0 24 24">
          <path d="M12 3v8" />
          <path d="M6.3 6.8a8 8 0 1 0 11.4 0" />
        </svg>
      }
      @case ('target') {
        <svg viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="7" />
          <path d="M12 2v6M12 16v6M2 12h6M16 12h6" />
        </svg>
      }
    }
  `,
  styles: `
    :host {
      display: inline-block;
      flex: none;
      width: 1.2em;
      height: 1.2em;
    }

    svg {
      display: block;
      width: 100%;
      height: 100%;
      fill: none;
      stroke: currentColor;
      stroke-width: 2.5;
      stroke-linecap: round;
    }
  `,
})
export class Icon {
  readonly name = input.required<IconName>();
}
