/** @type {import("prettier").Config} */
export default {
  plugins: ["prettier-plugin-tailwindcss", "prettier-plugin-svelte"],
  overrides: [
    {
      files: "*.svelte",
      options: {
        parser: "svelte",
      },
    },
  ],
};
