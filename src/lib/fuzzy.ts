import type { RepoDto } from "./types";

const MAX_QUERY_LEN = 256;

function subsequenceMatch(query: string, field: string): boolean {
  let qi = 0;
  for (let fi = 0; fi < field.length && qi < query.length; fi++) {
    if (field[fi] === query[qi]) {
      qi += 1;
    }
  }
  return qi === query.length;
}

function scoreField(query: string, field: string, nameBonus: boolean): number {
  const f = field.toLowerCase();
  const matches =
    f.includes(query) || subsequenceMatch(query, f);
  if (!matches) {
    return 0;
  }
  let score = 10;
  if (f.startsWith(query)) {
    score += 20;
  } else if (subsequenceMatch(query, f)) {
    score += 8;
  }
  if (nameBonus) {
    let run = 0;
    for (let i = 0; i < Math.min(query.length, f.length); i++) {
      if (f[i] === query[i]) {
        run += 1;
      } else {
        break;
      }
    }
    score += run * 2;
  }
  return score;
}

export function fuzzyScore(query: string, repo: RepoDto): number {
  const q = query.trim().toLowerCase();
  if (q.length === 0) {
    return 1;
  }
  if (q.length > MAX_QUERY_LEN) {
    return 0;
  }

  const scores = [
    scoreField(q, repo.name, true),
    scoreField(q, repo.path, false),
    scoreField(q, repo.branch ?? "", false),
  ];
  return Math.max(...scores);
}

export function fuzzyMatch(query: string, repo: RepoDto): boolean {
  return fuzzyScore(query, repo) > 0;
}
