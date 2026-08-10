const SYNC_TIMEOUT_MS = 90_000;

let watchdog: ReturnType<typeof setTimeout> | null = null;

export function armSyncWatchdog(onTimeout: () => void): void {
  clearSyncWatchdog();
  watchdog = setTimeout(() => {
    watchdog = null;
    onTimeout();
  }, SYNC_TIMEOUT_MS);
}

export function clearSyncWatchdog(): void {
  if (watchdog !== null) {
    clearTimeout(watchdog);
    watchdog = null;
  }
}
