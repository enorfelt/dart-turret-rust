import { VisionFace } from '../protocol/vision-stream';
import { OVERLAY_TUNING, describeFaces, labelTop, placeTorsoMarker, torsoHint } from './overlay';

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

  it('keeps a torso marker that fits in the frame where it is', () => {
    expect(placeTorsoMarker(face('unknown'), 320, 240)).toEqual({
      ...torsoHint(face('unknown')),
      offscreen: false,
    });
  });

  it('clamps a torso marker below the frame to the bottom edge', () => {
    const close = { ...face('target'), y: 60, height: 140 };

    expect(placeTorsoMarker(close, 320, 240)).toEqual({
      x: 120,
      y: 240 - OVERLAY_TUNING.edgeMargin,
      offscreen: true,
    });
  });

  it('keeps a clamped torso marker inside the frame sideways', () => {
    const atEdge = { ...face('unknown'), x: 300, width: 40, y: 200 };

    expect(placeTorsoMarker(atEdge, 320, 240).x).toBe(320 - OVERLAY_TUNING.edgeMargin);
  });

  it('puts the label above the box, or inside it at the top edge', () => {
    expect(labelTop(face('unknown'))).toBe(40 - OVERLAY_TUNING.labelHeight);
    expect(labelTop({ ...face('unknown'), y: 4 })).toBe(4);
  });

  it('describes faces for screen readers', () => {
    expect(describeFaces([])).toBe('no faces');
    expect(describeFaces([face('unknown')])).toBe('1 face: 1 unknown');
    expect(describeFaces([face('unknown'), face('target'), face('unknown')])).toBe(
      '3 faces: 1 target, 2 unknown',
    );
  });
});
