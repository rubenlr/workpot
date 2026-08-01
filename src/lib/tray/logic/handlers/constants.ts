export const SECTION_META = [
  { key: "pinned" as const, label: "Pinned", draggable: true },
  { key: "dirty" as const, label: "Dirty", draggable: false },
  { key: "recent" as const, label: "Recent", draggable: false },
  { key: "rest" as const, label: "Rest", draggable: false },
] as const;

export const DEFAULT_MAX_VISIBLE_ROWS = 15;

export const TRAY_EMPTY_LIST_MESSAGE = "No repos indexed yet.";
export const TRAY_NO_MATCH_MESSAGE = "No repos match";
