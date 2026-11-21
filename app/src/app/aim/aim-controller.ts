import { AimCommand, GimbalPosition } from '../protocol/turret-protocol';

export interface Deflection {
  x: number;
  y: number;
}

export interface AxisRange {
  min: number;
  max: number;
}

export interface AimTuning {
  deadzone: number;
  expo: number;
  maxRate: number;
  pan: AxisRange;
  tilt: AxisRange;
  sendIntervalMs: number;
  minSpeed: number;
  maxSpeed: number;
  invertPan: boolean;
  invertTilt: boolean;
}

export const AIM_TUNING: AimTuning = {
  deadzone: 0.08,
  expo: 2,
  maxRate: 600,
  pan: { min: 0, max: 990 },
  tilt: { min: 375, max: 600 },
  sendIntervalMs: 50,
  minSpeed: 50,
  maxSpeed: 1000,
  invertPan: false,
  invertTilt: true,
};

const CENTER: GimbalPosition = { pan: 512, tilt: 512 };

export class AimController {
  private position: GimbalPosition | null = null;
  private sent: GimbalPosition | null = null;
  private lastStepAt = 0;
  private lastSentAt = -Infinity;
  private rates = { pan: 0, tilt: 0 };

  constructor(private readonly tuning: AimTuning = AIM_TUNING) {}

  begin(start: GimbalPosition | null, nowMs: number): void {
    this.position = { ...(start ?? CENTER) };
    this.sent = start ? { ...start } : null;
    this.lastStepAt = nowMs;
    this.lastSentAt = -Infinity;
    this.rates = { pan: 0, tilt: 0 };
  }

  step(deflection: Deflection, nowMs: number): AimCommand | null {
    if (!this.position) return null;
    const seconds = (nowMs - this.lastStepAt) / 1000;
    this.lastStepAt = nowMs;
    const { invertPan, invertTilt } = this.tuning;
    this.rates = {
      pan: this.rate(deflection.x) * (invertPan ? -1 : 1),
      tilt: this.rate(deflection.y) * (invertTilt ? -1 : 1),
    };
    this.position = {
      pan: clamp(this.position.pan + this.rates.pan * seconds, this.tuning.pan),
      tilt: clamp(this.position.tilt + this.rates.tilt * seconds, this.tuning.tilt),
    };
    if (nowMs - this.lastSentAt < this.tuning.sendIntervalMs) return null;
    const command = this.unsentCommand();
    if (command) this.lastSentAt = nowMs;
    return command;
  }

  end(): AimCommand | null {
    const command = this.unsentCommand();
    this.position = null;
    return command;
  }

  private unsentCommand(): AimCommand | null {
    if (!this.position) return null;
    const target = { pan: Math.round(this.position.pan), tilt: Math.round(this.position.tilt) };
    if (this.sent?.pan === target.pan && this.sent.tilt === target.tilt) return null;
    this.sent = target;
    return {
      ...target,
      panSpeed: this.speed(this.rates.pan),
      tiltSpeed: this.speed(this.rates.tilt),
    };
  }

  private rate(deflection: number): number {
    const { deadzone, expo, maxRate } = this.tuning;
    const magnitude = Math.min(Math.abs(deflection), 1);
    if (magnitude < deadzone) return 0;
    const shaped = ((magnitude - deadzone) / (1 - deadzone)) ** expo;
    return Math.sign(deflection) * shaped * maxRate;
  }

  private speed(rate: number): number {
    const { minSpeed, maxSpeed } = this.tuning;
    return Math.round(Math.min(Math.max(Math.abs(rate), minSpeed), maxSpeed));
  }
}

function clamp(value: number, { min, max }: AxisRange): number {
  return Math.min(Math.max(value, min), max);
}
