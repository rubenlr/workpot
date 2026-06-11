export type TrayListView =
  | { kind: "error"; message: string }
  | { kind: "empty-list" }
  | { kind: "no-match" }
  | { kind: "list" };

/** Which tray list body to render (UAT #2, #3). */
export function trayListView(
  error: string | null,
  reposLength: number,
  filterQuery: string,
  displayLength: number,
): TrayListView {
  if (error) {
    return { kind: "error", message: error };
  }
  if (reposLength === 0) {
    return { kind: "empty-list" };
  }
  if (filterQuery.trim().length > 0 && displayLength === 0) {
    return { kind: "no-match" };
  }
  return { kind: "list" };
}
