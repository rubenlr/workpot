/** esbuild 0.28 cannot downlevel destructuring to Vite's default browser targets. */
export const esbuildTarget = "es2025";

export function applyEsbuildTarget<T extends import("vite").UserConfig>(
  config: T,
): T {
  config.build ??= {};
  config.build.target = esbuildTarget;
  if (config.esbuild !== false) {
    config.esbuild ??= {};
    config.esbuild.target = esbuildTarget;
  }
  config.optimizeDeps ??= {};
  config.optimizeDeps.esbuildOptions ??= {};
  config.optimizeDeps.esbuildOptions.target = esbuildTarget;
  return config;
}
