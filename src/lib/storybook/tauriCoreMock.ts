/** Storybook stub — no Tauri runtime. */
export async function invoke(cmd: string): Promise<unknown> {
  if (cmd === "list_branches") {
    return ["main", "develop"];
  }
  return undefined;
}

export async function listen(): Promise<() => void> {
  return () => {};
}
