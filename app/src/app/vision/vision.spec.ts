import { TestBed } from '@angular/core/testing';
import { FakeVisionLink } from './fake-vision-link';
import { VISION_LINK } from './vision-link';
import {
  DEFAULT_VISION_ADDRESS,
  RETRY_AFTER_MS,
  Vision,
  isStale,
  isValidVisionAddress,
} from './vision';

const FRAME = [
  0x01, 0x01, 0x02, 0x01, 0x40, 0x01, 0xf0, 0x00, 0x01, 0x07, 0x02, 0x53, 0x64, 0x00, 0x32, 0x00,
  0x3c, 0x00, 0x50, 0x00, 0xff, 0xd8,
];

describe('Vision', () => {
  let link: FakeVisionLink;

  function create(): Vision {
    TestBed.configureTestingModule({ providers: [{ provide: VISION_LINK, useValue: link }] });
    return TestBed.inject(Vision);
  }

  beforeEach(() => {
    localStorage.clear();
    link = new FakeVisionLink();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts idle on the default camera address', () => {
    const vision = create();

    expect(vision.connection()).toBe('idle');
    expect(vision.address()).toBe(DEFAULT_VISION_ADDRESS);
    expect(link.opened).toEqual([]);
  });

  it('opens the stream and goes live on the first frame', () => {
    const vision = create();
    vision.connect();

    expect(link.opened).toEqual(['ws://192.168.71.1/stream']);
    expect(vision.connection()).toBe('connecting');

    link.send(FRAME);

    expect(vision.connection()).toBe('live');
    expect(vision.frame()?.faces).toEqual([
      { track: 7, faceClass: 'target', confidence: 83, x: 100, y: 50, width: 60, height: 80 },
    ]);
    expect(vision.receivedAt()).not.toBeNull();
  });

  it('retries after the stream drops', () => {
    vi.useFakeTimers();
    const vision = create();
    vision.connect();
    link.send(FRAME);

    link.drop();

    expect(vision.connection()).toBe('lost');
    vi.advanceTimersByTime(RETRY_AFTER_MS);
    expect(link.opened).toHaveLength(2);
    expect(vision.connection()).toBe('connecting');
  });

  it('stops retrying once disconnected', () => {
    vi.useFakeTimers();
    const vision = create();
    vision.connect();
    link.drop();

    vision.disconnect();
    vi.advanceTimersByTime(RETRY_AFTER_MS * 3);

    expect(link.opened).toHaveLength(1);
    expect(vision.connection()).toBe('idle');
    expect(vision.frame()).toBeNull();
  });

  it('refuses a camera with an unknown stream version', () => {
    const vision = create();
    vision.connect();

    link.send([2, ...FRAME.slice(1)]);

    expect(vision.connection()).toBe('idle');
    expect(vision.error()).toContain('stream version 2');
  });

  it('remembers a valid address and reconnects to it', () => {
    const vision = create();
    vision.connect();

    expect(vision.setAddress(' 10.0.0.7:8080 ')).toBe(true);

    expect(link.opened.at(-1)).toBe('ws://10.0.0.7:8080/stream');
    expect(localStorage.getItem('dart-vision-address')).toBe('10.0.0.7:8080');
  });

  it('rejects addresses that are not a host and port', () => {
    const vision = create();

    expect(vision.setAddress('evil.example/path')).toBe(false);
    expect(vision.address()).toBe(DEFAULT_VISION_ADDRESS);
  });

  it('validates addresses and staleness', () => {
    expect(isValidVisionAddress('192.168.71.1')).toBe(true);
    expect(isValidVisionAddress('dart-vision.local')).toBe(true);
    expect(isValidVisionAddress('')).toBe(false);
    expect(isValidVisionAddress('a b')).toBe(false);
    expect(isStale(null, 1000)).toBe(true);
    expect(isStale(1000, 1500)).toBe(false);
    expect(isStale(1000, 1501)).toBe(true);
  });
});
