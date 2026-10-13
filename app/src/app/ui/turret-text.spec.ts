import { eventText, phaseText, speedText } from './turret-text';

describe('turret text', () => {
  it('names fire phases', () => {
    expect(phaseText('spin-up')).toBe('Spinning up');
    expect(phaseText('pushing')).toBe('Firing');
  });

  it('shows a dash without a light gate reading', () => {
    expect(speedText(72)).toBe('72 km/h');
    expect(speedText(null)).toBe('—');
  });

  it('describes events', () => {
    expect(eventText({ kind: 'shot', number: 2, speedKmh: 68 })).toBe('Shot 2: 68 km/h');
    expect(eventText({ kind: 'finished', shots: 1 })).toBe('Sequence finished after 1 shot');
    expect(eventText({ kind: 'finished', shots: 3 })).toBe('Sequence finished after 3 shots');
    expect(eventText({ kind: 'aborted' })).toBe('Sequence stopped by disarm');
    expect(eventText({ kind: 'fire-rejected', reason: 'cooling-down' })).toBe(
      'Did not fire: the flywheels are cooling down',
    );
    expect(eventText({ kind: 'move-failed' })).toBe('The gimbal did not accept the move');
  });
});
