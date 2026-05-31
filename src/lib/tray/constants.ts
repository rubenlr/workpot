export const SECTION_META = [
  { key: "pinned" as const, label: "Pinned", draggable: true },
  { key: "dirty" as const, label: "Dirty", draggable: false },
  { key: "recent" as const, label: "Recent", draggable: false },
  { key: "rest" as const, label: "Rest", draggable: false },
] as const;

export const DEFAULT_MAX_VISIBLE_ROWS = 15;
