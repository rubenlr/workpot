export const SECTION_META = [
  { key: "pinned", label: "Pinned", draggable: true },
  { key: "dirty", label: "Dirty", draggable: false },
  { key: "recent", label: "Recent", draggable: false },
  { key: "rest", label: "Rest", draggable: false },
  { key: "remote", label: "Not cloned", draggable: false },
] as const;

export const DEFAULT_MAX_VISIBLE_ROWS = 15;

export const TRAY_EMPTY_LIST_MESSAGE = "No repos in catalog yet.";
export const TRAY_NO_MATCH_MESSAGE = "No repos match";
