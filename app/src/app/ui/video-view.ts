import {
  Component,
  DestroyRef,
  ElementRef,
  computed,
  effect,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { FaceClass, VisionFrame } from '../protocol/vision-stream';
import { OVERLAY_TUNING, describeFaces, torsoHint } from '../vision/overlay';
import { Vision, isStale } from '../vision/vision';

const CLASS_COLORS: Record<FaceClass, string> = {
  unknown: '--hazard',
  protected: '--safe',
  target: '--fire',
};

@Component({
  selector: 'app-video-view',
  template: `
    @if (vision.active()) {
      <div class="frame" [class.stale]="stale()">
        <canvas #canvas width="320" height="240" role="img" [attr.aria-label]="label()"></canvas>
        @if (notice(); as notice) {
          <p class="notice" aria-live="polite">{{ notice }}</p>
        }
      </div>
    }
    @if (vision.error(); as error) {
      <p class="error" role="alert">{{ error }}</p>
    }
    <div class="controls">
      @if (vision.active()) {
        <button type="button" (click)="vision.disconnect()">Hide camera</button>
      } @else {
        <button type="button" (click)="vision.connect()">Show camera</button>
      }
      <label>
        <span>Camera</span>
        <input
          [value]="vision.address()"
          [attr.aria-invalid]="addressInvalid()"
          inputmode="url"
          autocomplete="off"
          spellcheck="false"
          (change)="setAddress($event)"
        />
      </label>
    </div>
  `,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      gap: 8px;
    }

    .frame {
      position: relative;
      border-radius: var(--radius-control);
      overflow: hidden;
      background: #000;
    }

    canvas {
      display: block;
      width: 100%;
      height: auto;
      aspect-ratio: 4 / 3;
    }

    .stale canvas {
      filter: grayscale(1);
      opacity: 0.5;
    }

    .notice {
      position: absolute;
      inset: auto 8px 8px;
      margin: 0;
      padding: 4px 8px;
      border-radius: 6px;
      background: rgb(0 0 0 / 70%);
      font-family: var(--font-label);
      font-size: 1.125rem;
      text-align: center;
    }

    .error {
      margin: 0;
      color: var(--fire);
    }

    .controls {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 8px 12px;
    }

    button {
      min-height: 40px;
      padding: 0 14px;
      border: 2px solid var(--panel-raised);
      border-radius: var(--radius-control);
      background: transparent;
    }

    label {
      display: flex;
      flex: 1;
      align-items: center;
      gap: 8px;
      min-width: 0;
      color: var(--ink-muted);
      font-size: 0.875rem;
    }

    input {
      flex: 1;
      min-width: 0;
      min-height: 40px;
      padding: 0 10px;
      border: 2px solid var(--panel-raised);
      border-radius: var(--radius-control);
      background: var(--panel);
      color: var(--ink);
      font: inherit;
    }

    input[aria-invalid='true'] {
      border-color: var(--fire);
    }
  `,
})
export class VideoView {
  protected readonly vision = inject(Vision);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly canvas = viewChild<ElementRef<HTMLCanvasElement>>('canvas');
  private readonly now = signal(performance.now());
  private drawing = false;
  private pending: VisionFrame | null = null;

  protected readonly addressInvalid = signal(false);
  protected readonly stale = computed(
    () => this.vision.connection() === 'live' && isStale(this.vision.receivedAt(), this.now()),
  );
  protected readonly label = computed(() => {
    const frame = this.vision.frame();
    return frame ? `Camera view, ${describeFaces(frame.faces)}` : 'Camera view';
  });
  protected readonly notice = computed(() => {
    switch (this.vision.connection()) {
      case 'connecting':
        return `Connecting to ${this.vision.address()}…`;
      case 'lost':
        return 'Camera lost, retrying…';
      default:
        return this.stale() ? 'Stale' : null;
    }
  });

  constructor() {
    const ticker = setInterval(() => this.now.set(performance.now()), 250);
    inject(DestroyRef).onDestroy(() => clearInterval(ticker));
    effect(() => {
      const frame = this.vision.frame();
      if (frame && this.canvas()) this.draw(frame);
    });
  }

  protected setAddress(event: Event): void {
    const input = event.target as HTMLInputElement;
    this.addressInvalid.set(!this.vision.setAddress(input.value));
  }

  private async draw(frame: VisionFrame): Promise<void> {
    if (this.drawing) {
      this.pending = frame;
      return;
    }
    this.drawing = true;
    try {
      const bitmap = await createImageBitmap(new Blob([frame.jpeg], { type: 'image/jpeg' }));
      this.paint(frame, bitmap);
      bitmap.close();
    } catch {
      // A corrupt JPEG only costs one frame.
    } finally {
      this.drawing = false;
      const next = this.pending;
      this.pending = null;
      if (next) void this.draw(next);
    }
  }

  private paint(frame: VisionFrame, bitmap: ImageBitmap): void {
    const canvas = this.canvas()?.nativeElement;
    const context = canvas?.getContext('2d');
    if (!canvas || !context) return;
    canvas.width = frame.width;
    canvas.height = frame.height;
    context.drawImage(bitmap, 0, 0, frame.width, frame.height);

    const style = getComputedStyle(this.host.nativeElement);
    const color = (name: string) => style.getPropertyValue(name).trim();
    context.lineWidth = 2;
    context.font = '12px sans-serif';
    for (const face of frame.faces) {
      context.strokeStyle = context.fillStyle = color(CLASS_COLORS[face.faceClass]);
      context.strokeRect(face.x, face.y, face.width, face.height);
      const label =
        face.faceClass === 'unknown' ? `#${face.track}` : `#${face.track} ${face.confidence}%`;
      context.fillText(label, face.x + 2, Math.max(12, face.y - 4));

      const torso = torsoHint(face);
      const r = OVERLAY_TUNING.torsoMarkerRadius;
      context.setLineDash([3, 3]);
      context.beginPath();
      context.arc(torso.x, torso.y, r, 0, Math.PI * 2);
      context.moveTo(torso.x - r * 2, torso.y);
      context.lineTo(torso.x + r * 2, torso.y);
      context.stroke();
      context.setLineDash([]);
    }

    const cx = frame.width / 2;
    const cy = frame.height / 2;
    const r = OVERLAY_TUNING.crosshairRadius;
    context.strokeStyle = color('--ink');
    context.beginPath();
    context.moveTo(cx - r * 2, cy);
    context.lineTo(cx - r / 2, cy);
    context.moveTo(cx + r / 2, cy);
    context.lineTo(cx + r * 2, cy);
    context.moveTo(cx, cy - r * 2);
    context.lineTo(cx, cy - r / 2);
    context.moveTo(cx, cy + r / 2);
    context.lineTo(cx, cy + r * 2);
    context.stroke();
  }
}
