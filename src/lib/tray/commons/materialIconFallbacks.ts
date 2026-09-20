/** UTF-8 stand-ins when Material Symbols Outlined is unavailable. */
export const MATERIAL_ICON_FALLBACKS: Readonly<Record<string, string>> = {
  arrow_back: "←",
  check: "✓",
  chevron_right: "›",
  folder: "📁",
  keep: "📌",
  north: "↑",
  search: "⌕",
  south: "↓",
  star: "★",
  sync: "↻",
};

export function materialIconFallback(name: string, filled = false): string {
  if (name === "keep") return filled ? "📌" : "📍";
  if (name === "star") return filled ? "★" : "☆";
  return MATERIAL_ICON_FALLBACKS[name] ?? "•";
}
