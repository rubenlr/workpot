<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, fireEvent, waitFor } from "storybook/test";
  import TrayPanelChromeFilterHarness from "./TrayPanelChromeFilterHarness.svelte";

  const { Story } = defineMeta({
    title: "Composites/Panel",
    component: TrayPanelChromeFilterHarness,
    tags: ["autodocs"],
    parameters: { layout: "fullscreen" },
  });
</script>

<Story
  name="FilterNarrowsList"
  play={async ({ canvas, userEvent }) => {
    const input = canvas.getByPlaceholderText(
      "Filter repos…",
    ) as HTMLInputElement;
    await userEvent.click(input);
    const setValue = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )?.set;
    setValue?.call(input, "alpha");
    await fireEvent.input(input);
    await waitFor(() => {
      expect(canvas.queryByRole("button", { name: /Open beta/ })).toBeNull();
    });
    await expect(
      canvas.getByRole("button", { name: /Open alpha/ }),
    ).toBeVisible();
  }}
/>
