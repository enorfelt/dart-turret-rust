import { FirePhase, Rejection, TurretEvent } from '../protocol/turret-protocol';

const PHASE_TEXT: Record<FirePhase, string> = {
  idle: 'Idle',
  'spin-up': 'Spinning up',
  pushing: 'Firing',
  retracting: 'Reloading',
  'spin-down': 'Spinning down',
};

const REJECTION_TEXT: Record<Rejection, string> = {
  disarmed: 'the turret is disarmed',
  busy: 'a sequence is still running',
  'cooling-down': 'the flywheels are cooling down',
  'no-throttle': 'throttle is zero',
};

export function phaseText(phase: FirePhase): string {
  return PHASE_TEXT[phase];
}

export function speedText(speedKmh: number | null): string {
  return speedKmh === null ? '—' : `${speedKmh} km/h`;
}

export function eventText(event: TurretEvent): string {
  switch (event.kind) {
    case 'shot':
      return `Shot ${event.number}: ${speedText(event.speedKmh)}`;
    case 'finished':
      return `Sequence finished after ${event.shots} ${event.shots === 1 ? 'shot' : 'shots'}`;
    case 'aborted':
      return 'Sequence stopped by disarm';
    case 'fire-rejected':
      return `Did not fire: ${REJECTION_TEXT[event.reason]}`;
    case 'move-failed':
      return 'The gimbal did not accept the move';
  }
}
