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
import { FaceClass, VisionFace, VisionFrame } from '../protocol/vision-stream';
import {
  OVERLAY_TUNING,
  TorsoMarker,
  describeFaces,
  labelTop,
  placeTorsoMarker,
} from '../vision/overlay';
import { Vision, isStale } from '../vision/vision';
import { enterFullscreen } from './fullscreen';

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
    } @else {
      @if (vision.error(); as error) {
        <p class="error" role="alert">{{ error }}</p>
      }
      <div class="controls">
        <button type="button" (click)="showCamera()">Show camera</button>
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
    }
  `,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      gap: 8px;
    }

    .frame {
      position: relative;
      flex: 1;
      min-height: 0;
      background: #000;
    }

    canvas {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: contain;
    }

    .stale canvas {
      filter: grayscale(1);
      opacity: 0.5;
    }

    .notice {
      position: absolute;
      bottom: 12px;
      left: 50%;
      width: max-content;
      max-width: min(calc(100% - 16px), 36ch);
      transform: translateX(-50%);
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
        return this.vision.error() ?? 'Camera lost, retrying…';
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

  protected showCamera(): void {
    void enterFullscreen();
    this.vision.connect();
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
    context.font = '12px sans-serif';
    context.textBaseline = 'top';
    for (const face of frame.faces) {
      const faceColor = color(CLASS_COLORS[face.faceClass]);
      outlined(context, faceColor, () => {
        context.beginPath();
        context.rect(face.x, face.y, face.width, face.height);
      });
      drawLabel(context, face, faceColor);
      drawTorsoMarker(context, placeTorsoMarker(face, frame.width, frame.height), faceColor);
    }
    drawCrosshair(context, frame.width / 2, frame.height / 2, color('--ink'));
  }
}

const OUTLINE = 'rgb(0 0 0 / 65%)';

function outlined(context: CanvasRenderingContext2D, stroke: string, path: () => void): void {
  path();
  context.lineWidth = 4;
  context.strokeStyle = OUTLINE;
  context.stroke();
  context.lineWidth = 2;
  context.strokeStyle = stroke;
  context.stroke();
}

function drawLabel(context: CanvasRenderingContext2D, face: VisionFace, fill: string): void {
  const text =
    face.faceClass === 'unknown' ? `#${face.track}` : `#${face.track} ${face.confidence}%`;
  const top = labelTop(face);
  context.fillStyle = OUTLINE;
  context.fillRect(face.x, top, context.measureText(text).width + 6, OVERLAY_TUNING.labelHeight);
  context.fillStyle = fill;
  context.fillText(text, face.x + 3, top + 2);
}

function drawTorsoMarker(
  context: CanvasRenderingContext2D,
  marker: TorsoMarker,
  stroke: string,
): void {
  const r = OVERLAY_TUNING.torsoMarkerRadius;
  if (marker.offscreen) {
    outlined(context, stroke, () => {
      context.beginPath();
      context.moveTo(marker.x - r, marker.y - r);
      context.lineTo(marker.x, marker.y);
      context.lineTo(marker.x + r, marker.y - r);
      context.moveTo(marker.x, marker.y);
      context.lineTo(marker.x, marker.y - r * 2);
    });
    return;
  }
  context.setLineDash([3, 3]);
  outlined(context, stroke, () => {
    context.beginPath();
    context.arc(marker.x, marker.y, r, 0, Math.PI * 2);
    context.moveTo(marker.x - r * 2, marker.y);
    context.lineTo(marker.x + r * 2, marker.y);
  });
  context.setLineDash([]);
}

function drawCrosshair(
  context: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  stroke: string,
): void {
  const r = OVERLAY_TUNING.crosshairRadius;
  outlined(context, stroke, () => {
    context.beginPath();
    context.moveTo(cx - r * 2, cy);
    context.lineTo(cx - r / 2, cy);
    context.moveTo(cx + r / 2, cy);
    context.lineTo(cx + r * 2, cy);
    context.moveTo(cx, cy - r * 2);
    context.lineTo(cx, cy - r / 2);
    context.moveTo(cx, cy + r / 2);
    context.lineTo(cx, cy + r * 2);
  });
}
