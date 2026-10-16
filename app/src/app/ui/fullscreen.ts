type LockableOrientation = ScreenOrientation & {
  lock?: (orientation: 'landscape') => Promise<void>;
};

export async function enterFullscreen(): Promise<void> {
  try {
    await document.documentElement.requestFullscreen?.({ navigationUI: 'hide' });
    await (screen.orientation as LockableOrientation | undefined)?.lock?.('landscape');
  } catch {
    // Full screen and the orientation lock are conveniences; the layout works without them.
  }
}

export async function exitFullscreen(): Promise<void> {
  if (!document.fullscreenElement) return;
  try {
    await document.exitFullscreen();
  } catch {
    // The browser may already have left full screen.
  }
}
