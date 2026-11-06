import type { RepoDto } from "./types";

const MAX_QUERY_LEN = 256;

/** Score of any matching field, before the bonuses below. */
const BASE_MATCH_SCORE = 10;
const PREFIX_MATCH_BONUS = 20;
const SUBSEQUENCE_MATCH_BONUS = 8;
/** Per leading character shared with the query; only for name-like fields. */
const NAME_PREFIX_RUN_WEIGHT = 2;

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
  // A substring is also a subsequence, so the subsequence test alone decides a match.
  if (!subsequenceMatch(query, f)) {
    return 0;
  }
  const matchBonus = f.startsWith(query)
    ? PREFIX_MATCH_BONUS
    : SUBSEQUENCE_MATCH_BONUS;
  let score = BASE_MATCH_SCORE + matchBonus;
  if (nameBonus) {
    let run = 0;
    for (let i = 0; i < Math.min(query.length, f.length); i++) {
      if (f[i] === query[i]) {
        run += 1;
      } else {
        break;
      }
    }
    score += run * NAME_PREFIX_RUN_WEIGHT;
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
    scoreField(q, repo.alias ?? "", true),
    scoreField(q, repo.path, false),
    scoreField(q, repo.branch ?? "", false),
    scoreField(q, repo.notes ?? "", false),
    scoreField(q, repo.remote_full_path ?? "", false),
    ...repo.tags.map((t) => scoreField(q, t, false)),
  ];
  return Math.max(...scores);
}

export function fuzzyMatch(query: string, repo: RepoDto): boolean {
  return fuzzyScore(query, repo) > 0;
}
