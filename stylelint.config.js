/** @type {import('stylelint').Config} */
export default {
  extends: [
    "stylelint-config-standard",
    "stylelint-config-html",
    "stylelint-config-tailwindcss",
  ],
  rules: {
    // Tailwind v4 @custom-variant uses nested & and allows @import after at-rules.
    "nesting-selector-no-missing-scoping-root": null,
    "no-invalid-position-at-import-rule": null,

    // Design tokens prefer readable rgba() and full hex values.
    "color-function-notation": null,
    "color-function-alias-notation": null,
    "alpha-value-notation": null,
    "color-hex-length": null,

    // Font stacks and CSS keywords keep conventional casing.
    "value-keyword-case": [
      "lower",
      {
        camelCaseSvgKeywords: true,
        ignoreKeywords: ["currentColor", "BlinkMacSystemFont"],
      },
    ],

    // Icon fonts (Material Symbols) omit generic fallbacks by design.
    "font-family-no-missing-generic-family-keyword": null,

    // Scoped Svelte selectors and utility-heavy markup are intentional.
    "selector-class-pattern": null,
    "no-descending-specificity": null,
  },
};
