/** Dev-only tray diagnostics (visible in the panel webview inspector console). */
export function trayTrace(message: string, detail?: unknown): void {
  if (!import.meta.env.DEV) {
    return;
  }
  if (detail !== undefined) {
    console.debug(`[workpot-tray] ${message}`, detail);
  } else {
    console.debug(`[workpot-tray] ${message}`);
  }
}
