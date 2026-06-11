const GIT_REFRESH_TIMEOUT_MS = 90_000;

let watchdog: ReturnType<typeof setTimeout> | null = null;

export function armGitRefreshWatchdog(onTimeout: () => void): void {
  clearGitRefreshWatchdog();
  watchdog = setTimeout(() => {
    watchdog = null;
    onTimeout();
  }, GIT_REFRESH_TIMEOUT_MS);
}

export function clearGitRefreshWatchdog(): void {
  if (watchdog !== null) {
    clearTimeout(watchdog);
    watchdog = null;
  }
}
