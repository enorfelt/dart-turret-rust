import { VisionFace } from '../protocol/vision-stream';
import { OVERLAY_TUNING, describeFaces, torsoHint } from './overlay';

const face = (faceClass: VisionFace['faceClass']): VisionFace => ({
  track: 1,
  faceClass,
  confidence: 0,
  x: 100,
  y: 40,
  width: 40,
  height: 60,
});

describe('overlay', () => {
  it('puts the torso hint below the face centre', () => {
    expect(torsoHint(face('unknown'))).toEqual({
      x: 120,
      y: 70 + OVERLAY_TUNING.torsoBelowFaceHeights * 60,
    });
  });

  it('describes faces for screen readers', () => {
    expect(describeFaces([])).toBe('no faces');
    expect(describeFaces([face('unknown')])).toBe('1 face: 1 unknown');
    expect(describeFaces([face('unknown'), face('target'), face('unknown')])).toBe(
      '3 faces: 1 target, 2 unknown',
    );
  });
});
