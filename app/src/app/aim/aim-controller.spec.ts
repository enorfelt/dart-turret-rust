import { AIM_TUNING, AimController, AimTuning } from './aim-controller';

const tuning: AimTuning = {
  ...AIM_TUNING,
  deadzone: 0.1,
  expo: 1,
  maxRate: 600,
  min: 0,
  max: 1023,
  sendIntervalMs: 50,
  minSpeed: 50,
  maxSpeed: 1000,
  invertPan: false,
  invertTilt: false,
};

function started(start = { pan: 500, tilt: 500 }, overrides: Partial<AimTuning> = {}) {
  const aim = new AimController({ ...tuning, ...overrides });
  aim.begin(start, 0);
  return aim;
}

describe('AimController', () => {
  it('moves at the full rate for a full deflection', () => {
    const aim = started();

    const command = aim.step({ x: 1, y: -1 }, 100);

    expect(command).toMatchObject({ pan: 560, tilt: 440 });
  });

  it('ignores deflection inside the deadzone', () => {
    const aim = started();

    expect(aim.step({ x: 0.05, y: -0.09 }, 100)).toBeNull();
  });

  it('scales the rate from the deadzone edge, shaped by expo', () => {
    const linear = started().step({ x: 0.55, y: 0 }, 100);
    const curved = started(undefined, { expo: 2 }).step({ x: 0.55, y: 0 }, 100);

    expect(linear?.pan).toBe(530);
    expect(curved?.pan).toBe(515);
  });

  it('clamps to the gimbal range', () => {
    const aim = started({ pan: 1010, tilt: 5 });

    expect(aim.step({ x: 1, y: -1 }, 100)).toMatchObject({ pan: 1023, tilt: 0 });
  });

  it('starts at the center when the turret has not moved yet', () => {
    const aim = new AimController(tuning);
    aim.begin(null, 0);

    expect(aim.step({ x: 1, y: 0 }, 100)).toMatchObject({ pan: 572, tilt: 512 });
  });

  it('sends at most once per interval and integrates in between', () => {
    const aim = started();

    expect(aim.step({ x: 1, y: 0 }, 50)).toMatchObject({ pan: 530 });
    expect(aim.step({ x: 1, y: 0 }, 70)).toBeNull();
    expect(aim.step({ x: 1, y: 0 }, 100)).toMatchObject({ pan: 560 });
  });

  it('flushes the last unsent position on end', () => {
    const aim = started();
    aim.step({ x: 1, y: 0 }, 50);
    aim.step({ x: 1, y: 0 }, 70);

    expect(aim.end()).toMatchObject({ pan: 542 });
    expect(aim.end()).toBeNull();
  });

  it('derives servo speed from the rate with a floor and a cap', () => {
    const slow = started().step({ x: 0.2, y: 0 }, 100);
    const fast = started(undefined, { maxRate: 5000 }).step({ x: 1, y: 0 }, 100);

    expect(slow).toMatchObject({ panSpeed: 67, tiltSpeed: 50 });
    expect(fast?.panSpeed).toBe(1000);
  });

  it('inverts axes when configured', () => {
    const aim = started(undefined, { invertPan: true, invertTilt: true });

    expect(aim.step({ x: 1, y: 1 }, 100)).toMatchObject({ pan: 440, tilt: 440 });
  });

  it('does nothing before begin', () => {
    expect(new AimController(tuning).step({ x: 1, y: 1 }, 100)).toBeNull();
  });
});
