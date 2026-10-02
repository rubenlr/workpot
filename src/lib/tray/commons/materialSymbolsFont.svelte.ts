const FONT_FACE = '24px "Material Symbols Outlined"';

/** Shared across MaterialIcon instances — false until the face is confirmed. */
let available = $state(false);
let probeStarted = false;

function markAvailable(): void {
  if (typeof document === "undefined" || !document.fonts) {
    available = false;
    return;
  }
  available = document.fonts.check(FONT_FACE);
}

/** Probe once; stay on UTF-8 fallbacks until the face resolves. */
export function probeMaterialSymbolsFont(): void {
  if (probeStarted) return;
  probeStarted = true;

  if (typeof document === "undefined" || !document.fonts) {
    available = false;
    return;
  }

  markAvailable();
  void document.fonts
    .load(FONT_FACE)
    .then(markAvailable)
    .catch(() => {
      available = false;
    });
  void document.fonts.ready.then(markAvailable);
}

export function materialSymbolsFontAvailable(): boolean {
  return available;
}

/**
 * Test-only: force availability and lock the probe so jsdom's missing
 * `document.fonts` cannot clobber the stub on mount.
 */
export function resetMaterialSymbolsFontProbeForTests(next = false): void {
  available = next;
  probeStarted = true;
}
