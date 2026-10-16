export const STREAM_VERSION = 1;
const HEADER_LENGTH = 9;
const FACE_LENGTH = 11;
const FLAG_RECOGNITION = 1 << 0;
const FLAG_ENROLLING = 1 << 1;

export type FaceClass = 'unknown' | 'protected' | 'target';

const CLASSES: readonly FaceClass[] = ['unknown', 'protected', 'target'];

export interface VisionFace {
  track: number;
  faceClass: FaceClass;
  confidence: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface VisionFrame {
  recognition: boolean;
  enrolling: boolean;
  seq: number;
  width: number;
  height: number;
  faces: VisionFace[];
  jpeg: ArrayBuffer;
}

export class StreamError extends Error {
  override name = 'StreamError';
}

export function decodeVisionFrame(buffer: ArrayBuffer): VisionFrame {
  const view = new DataView(buffer);
  if (view.byteLength < HEADER_LENGTH) throw new StreamError('Vision frame too short');
  const version = view.getUint8(0);
  if (version !== STREAM_VERSION) {
    throw new StreamError(
      `Camera speaks stream version ${version}, this app only knows ${STREAM_VERSION}`,
    );
  }
  const flags = view.getUint8(1);
  const count = view.getUint8(8);
  const end = HEADER_LENGTH + count * FACE_LENGTH;
  if (view.byteLength < end) throw new StreamError('Vision frame too short');

  const faces: VisionFace[] = [];
  for (let at = HEADER_LENGTH; at < end; at += FACE_LENGTH) {
    const faceClass = CLASSES[view.getUint8(at + 1)];
    if (!faceClass) throw new StreamError(`Unknown face class ${view.getUint8(at + 1)}`);
    faces.push({
      track: view.getUint8(at),
      faceClass,
      confidence: view.getUint8(at + 2),
      x: view.getUint16(at + 3, true),
      y: view.getUint16(at + 5, true),
      width: view.getUint16(at + 7, true),
      height: view.getUint16(at + 9, true),
    });
  }

  return {
    recognition: (flags & FLAG_RECOGNITION) !== 0,
    enrolling: (flags & FLAG_ENROLLING) !== 0,
    seq: view.getUint16(2, true),
    width: view.getUint16(4, true),
    height: view.getUint16(6, true),
    faces,
    jpeg: buffer.slice(end),
  };
}
