export function observePanelHeight(
  node: HTMLElement,
  onHeight?: (px: number) => void,
): { destroy: () => void } {
  if (!onHeight) {
    return { destroy() {} };
  }

  const reportHeight = () => {
    onHeight(Math.round(node.getBoundingClientRect().height));
  };

  if (typeof ResizeObserver === "undefined") {
    reportHeight();
    return { destroy() {} };
  }

  const observer = new ResizeObserver((entries) => {
    const entry = entries[0];
    if (!entry) return;
    onHeight(Math.round(entry.contentRect.height));
  });

  observer.observe(node);
  reportHeight();

  return {
    destroy() {
      observer.disconnect();
    },
  };
}
