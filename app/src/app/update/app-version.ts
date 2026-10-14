import { InjectionToken } from '@angular/core';

declare const APP_VERSION: string;

export const APP_VERSION_TOKEN = new InjectionToken<string>('APP_VERSION', {
  factory: () => APP_VERSION,
});
