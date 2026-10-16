import { StreamError, decodeVisionFrame } from './vision-stream';

// Same byte vector as `vision_core::stream` tests.
const FRAME = [
  0x01, 0x01, 0x02, 0x01, 0x40, 0x01, 0xf0, 0x00, 0x02, 0x07, 0x02, 0x53, 0x64, 0x00, 0x32, 0x00,
  0x3c, 0x00, 0x50, 0x00, 0x09, 0x01, 0x3d, 0x0a, 0x00, 0x14, 0x00, 0x1e, 0x00, 0x28, 0x00, 0xff,
  0xd8,
];

const buffer = (bytes: readonly number[]) => new Uint8Array(bytes).buffer;

describe('vision stream', () => {
  it('decodes header, faces and jpeg', () => {
    const frame = decodeVisionFrame(buffer(FRAME));

    expect({ ...frame, jpeg: [...new Uint8Array(frame.jpeg)] }).toEqual({
      recognition: true,
      enrolling: false,
      seq: 0x0102,
      width: 320,
      height: 240,
      faces: [
        { track: 7, faceClass: 'target', confidence: 83, x: 100, y: 50, width: 60, height: 80 },
        { track: 9, faceClass: 'protected', confidence: 61, x: 10, y: 20, width: 30, height: 40 },
      ],
      jpeg: [0xff, 0xd8],
    });
  });

  it('decodes a frame without faces', () => {
    const frame = decodeVisionFrame(buffer([0x01, 0x02, 0, 0, 0x40, 0x01, 0xf0, 0x00, 0x00]));

    expect(frame.faces).toEqual([]);
    expect(frame.enrolling).toBe(true);
    expect(frame.jpeg.byteLength).toBe(0);
  });

  it('rejects truncated frames', () => {
    expect(() => decodeVisionFrame(buffer(FRAME.slice(0, 8)))).toThrow(StreamError);
    expect(() => decodeVisionFrame(buffer(FRAME.slice(0, 20)))).toThrow(StreamError);
  });

  it('rejects other stream versions', () => {
    expect(() => decodeVisionFrame(buffer([2, ...FRAME.slice(1)]))).toThrow('stream version 2');
  });

  it('rejects unknown face classes', () => {
    const bad = [...FRAME];
    bad[10] = 5;

    expect(() => decodeVisionFrame(buffer(bad))).toThrow('face class 5');
  });
});
