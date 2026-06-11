import type { Preview } from "@storybook/sveltekit";
import "../src/app.css";

function syncPreviewTheme(background: unknown) {
  const root = document.documentElement;
  if (background === "light") {
    root.setAttribute("data-theme", "light");
  } else if (background === "dark") {
    root.setAttribute("data-theme", "dark");
  } else {
    root.removeAttribute("data-theme");
  }
}

const preview: Preview = {
  decorators: [
    (storyFn, context) => {
      syncPreviewTheme(context.globals.backgrounds?.value);
      return storyFn();
    },
  ],
  parameters: {
    layout: "padded",
    backgrounds: {
      default: "dark",
      values: [
        { name: "dark", value: "#0f0f10" },
        { name: "light", value: "#f5f5f7" },
      ],
    },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    a11y: {
      test: "todo",
    },
  },
};

export default preview;
