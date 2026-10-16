import { FaceClass, VisionFace } from '../protocol/vision-stream';

export const OVERLAY_TUNING = {
  torsoBelowFaceHeights: 1.5,
  torsoMarkerRadius: 6,
  crosshairRadius: 10,
} as const;

export interface Point {
  x: number;
  y: number;
}

export function torsoHint(face: VisionFace): Point {
  return {
    x: face.x + face.width / 2,
    y: face.y + face.height / 2 + OVERLAY_TUNING.torsoBelowFaceHeights * face.height,
  };
}

export function describeFaces(faces: readonly VisionFace[]): string {
  if (faces.length === 0) return 'no faces';
  const counts = new Map<FaceClass, number>();
  for (const face of faces) counts.set(face.faceClass, (counts.get(face.faceClass) ?? 0) + 1);
  const parts = (['target', 'protected', 'unknown'] as const)
    .filter((faceClass) => counts.has(faceClass))
    .map((faceClass) => `${counts.get(faceClass)} ${faceClass}`);
  return `${faces.length} ${faces.length === 1 ? 'face' : 'faces'}: ${parts.join(', ')}`;
}
