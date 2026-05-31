import type { RepoDto } from "./types";

type RepoWithPinOrder = RepoDto & { pin_order?: number | null };

export function reorderPinned(
  repos: RepoDto[],
  from: number,
  to: number,
): RepoWithPinOrder[] {
  if (from === to) {
    return repos as RepoWithPinOrder[];
  }

  const result = [...repos] as RepoWithPinOrder[];
  const [item] = result.splice(from, 1);
  result.splice(to, 0, item);

  return result.map((r, i) => ({ ...r, pin_order: i }));
}

export function toPinOrderPayload(
  repos: RepoDto[],
): Array<{ path: string; order: number }> {
  return repos.map((r, i) => ({ path: r.path, order: i }));
}
