/**
 * Frontend files excluded from Vitest coverage and Sonar coverage metrics.
 * Non-testable: types, constants, Storybook fixtures/mocks, dev trace, route shells.
 *
 * Keep sonar-project.properties sonar.coverage.exclusions in sync.
 */
export const nonTestableCoverageGlobs = [
  "src/lib/types.ts",
  "src/lib/tray/constants.ts",
  "src/lib/tray/storybook/**",
  "src/lib/tray/repo-list/repoStoryFixtures.ts",
  "src/lib/storybook/**",
  "**/*.stories.svelte",
  "src/lib/tray/trayTrace.ts",
  "src/routes/+page.svelte",
  "src/routes/+layout.svelte",
  "src/routes/+layout.ts",
];
