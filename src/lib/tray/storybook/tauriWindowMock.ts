/** Storybook stub — no Tauri window runtime. */
export function getCurrentWindow() {
  return {
    hide: async () => {},
    show: async () => {},
    setSize: async () => {},
    setFocus: async () => {},
  };
}

export function getAllWindows() {
  return [getCurrentWindow()];
}
