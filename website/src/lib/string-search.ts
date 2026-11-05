/* SPDX-FileCopyrightText: 2023 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/**
 * Blender's fuzzy string search (blenlib `BLI_string_search`), ported to TypeScript from
 * the repository's copy in core/intern/string_search.cc. It is what Blender's menu
 * search (F3) uses, and what BUI's kit uses for menu search and search fields.
 *
 * Per query word, in order of preference:
 *   1. a word of the item starts with it          ("cu"   -> "Cube")         +10 / +9
 *   2. it matches the initials of several words   ("msfv" -> "Mark Sharp from Vertices")  +4 / +3
 *   3. a word matches with a few typos            ("tkintr" -> "tkinter")    +3 - errors
 * Every query word must match, words out of order cost 1, and results with the best
 * score are sorted by weight, then shorter first, then recently picked first.
 *
 * Items can be split into groups with " ▸ " ("Python ▸ Qt syntax ▸ Signals"); words in the
 * main group (the last one, with `mainWords: "last"`) count a little more.
 *
 * Port notes (BUI web): strings are JavaScript strings (UTF-16 code points via Array.from)
 * instead of UTF-8 bytes; everything else follows the C++ line for line where it can.
 */

const ARROW_SEP = "▸"; // ▸, Blender's UI_MENU_ARROW_SEP
const UNUSED_WORD = -1;

export function damerauLevenshteinDistance(a: string, b: string): number {
  const ca = Array.from(a);
  const cb = Array.from(b);
  const rowLength = cb.length + 1;
  let v0 = new Array<number>(rowLength).fill(0);
  let v1 = new Array<number>(rowLength);
  let v2 = new Array<number>(rowLength).fill(0);
  for (let i = 0; i < rowLength; i++) v1[i] = i;

  for (let i = 0; i < ca.length; i++) {
    v2[0] = i + 1;
    for (let j = 0; j < cb.length; j++) {
      let cost = Math.min(v1[j + 1] + 1, v2[j] + 1, v1[j] + (ca[i] !== cb[j] ? 1 : 0));
      if (i > 0 && j > 0 && ca[i] === cb[j - 1] && ca[i - 1] === cb[j]) {
        cost = Math.min(cost, v0[j - 1] + 1);
      }
      v2[j + 1] = cost;
    }
    [v0, v1, v2] = [v1, v2, v0];
  }
  return v1[rowLength - 1];
}

/** Number of errors when `query` fuzzily matches a part of `full`, or -1 when it doesn't. */
export function getFuzzyMatchErrors(query: string, full: string): number {
  if (full.includes(query)) return 0;

  const q = Array.from(query);
  const f = Array.from(full);
  if (q.length === 1) return -1;

  const maxErrors = q.length <= 1 ? 0 : Math.floor(q.length / 8) + 1;
  if (q.length - f.length > maxErrors) return -1;

  const windowSize = Math.min(q.length + maxErrors, f.length);
  const extraChars = Math.max(windowSize - q.length, 0);
  const maxAcceptableDistance = maxErrors + extraChars;

  let begin = 0;
  let end = windowSize;
  for (;;) {
    let distance = 0;
    // Expect the first or second character of the query to be right; it saves work.
    if (f[begin] === q[0] || f[begin] === q[1]) {
      distance = damerauLevenshteinDistance(query, f.slice(begin, end).join(""));
      if (distance <= maxAcceptableDistance) return distance;
    }
    if (end === f.length) return -1;
    // A large distance can't shrink quickly: skip ahead a few characters.
    const offset = Math.max(1, Math.floor(distance / 2));
    for (let i = 0; i < offset && end < f.length; i++) {
      begin++;
      end++;
    }
  }
}

export type MainWords = "first" | "last" | "all";

interface Item<T> {
  data: T;
  words: string[];
  groups: number[];
  mainGroup: number;
  mainGroupLength: number;
  totalLength: number;
  weight: number;
  recentTime: number;
  deprecated: boolean;
}

/** Splits at spaces, -, _, / and ▸ (which also starts a new group), lower-cased. */
export function extractNormalizedWords(str: string): { words: string[]; groups: number[] } {
  const words: string[] = [];
  const groups: number[] = [];
  const lower = str.toLowerCase();
  let group = 0;
  let current = "";
  for (const ch of lower) {
    if (ch === " " || ch === "-" || ch === "_" || ch === "/" || ch === ARROW_SEP) {
      if (current) {
        words.push(current);
        groups.push(group);
        current = "";
      }
      if (ch === ARROW_SEP) group++;
    } else {
      current += ch;
    }
  }
  if (current) {
    words.push(current);
    groups.push(group);
  }
  return { words, groups };
}

const countMainGroupMatches = (indices: number[], item: Item<unknown>) =>
  indices.filter((i) => item.groups[i] === item.mainGroup).length;

/**
 * Matches the query against the first characters of words, in order, skipping words:
 * "msfv" -> "Mark Sharp from Vertices", "seboulo" -> "select boundary loop".
 */
function matchWordInitials(query: string, item: Item<unknown>, wordMatchMap: number[], start = 0): number[] | null {
  const words = item.words;
  if (start >= words.length) return null;
  const matched: number[] = [];
  const q = Array.from(query);
  let wordIndex = start;
  let charIndex = 0;
  let firstFound = -1;

  for (const qc of q) {
    for (;;) {
      if (wordIndex >= words.length) {
        if (firstFound >= 0) return matchWordInitials(query, item, wordMatchMap, firstFound + 1);
        return null;
      }
      if (wordMatchMap[wordIndex] !== UNUSED_WORD) {
        wordIndex++;
        continue;
      }
      const word = Array.from(words[wordIndex]);
      if (charIndex < word.length) {
        const wc = word[charIndex++];
        if (qc === wc) {
          matched.push(wordIndex);
          if (firstFound === -1) firstFound = wordIndex;
          break;
        }
      }
      wordIndex++;
      charIndex = 0;
    }
  }
  const sub = matchWordInitials(query, item, wordMatchMap, firstFound + 1);
  if (sub && countMainGroupMatches(sub, item) > countMainGroupMatches(matched, item)) return sub;
  return matched;
}

function bestWordThatStartsWith(query: string, item: Item<unknown>, wordMatchMap: number[], remaining: string[]): number {
  // If a later query word also starts with this one, take the shortest word so the later
  // one still finds its match ("t test" must match "Test").
  const useShortest = remaining.some((w) => w.startsWith(query));
  let bestSize = Number.MAX_SAFE_INTEGER;
  let best = -1;
  let bestInMain = false;
  item.words.forEach((word, i) => {
    if (wordMatchMap[i] !== UNUSED_WORD || !word.startsWith(query)) return;
    const inMain = item.groups[i] === item.mainGroup;
    const better = useShortest ? word.length < bestSize : !bestInMain;
    if (better) {
      best = i;
      bestSize = word.length;
      bestInMain = inMain;
    }
  });
  return best;
}

function scoreQueryAgainstWords(queryWords: string[], item: Item<unknown>): number | null {
  const wordMatchMap = new Array<number>(item.words.length).fill(UNUSED_WORD);
  let total = item.deprecated ? 500 : 1000;

  for (let qi = 0; qi < queryWords.length; qi++) {
    const qw = queryWords[qi];
    const w = bestWordThatStartsWith(qw, item, wordMatchMap, queryWords.slice(qi + 1));
    if (w >= 0) {
      total += item.groups[w] === item.mainGroup ? 10 : 9;
      wordMatchMap[w] = qi;
      continue;
    }
    const initials = matchWordInitials(qw, item, wordMatchMap);
    if (initials) {
      total += countMainGroupMatches(initials, item) === initials.length ? 4 : 3;
      for (const i of initials) wordMatchMap[i] = qi;
      continue;
    }
    let fuzzy = -1;
    let errors = 0;
    for (let i = 0; i < item.words.length; i++) {
      if (wordMatchMap[i] !== UNUSED_WORD) continue;
      const e = getFuzzyMatchErrors(qw, item.words[i]);
      if (e >= 0) {
        fuzzy = i;
        errors = e;
        break;
      }
    }
    if (fuzzy >= 0) {
      total += 3 - errors;
      wordMatchMap[fuzzy] = qi;
      continue;
    }
    return null;
  }

  // Penalty for query words matched out of order.
  const order = wordMatchMap.filter((i) => i !== UNUSED_WORD);
  for (let i = 0; i + 1 < order.length; i++) if (order[i] > order[i + 1]) total -= 1;
  return total;
}

/** Recently picked strings, newest with the highest logical time. */
export interface RecentCache {
  get(str: string): number | undefined;
}

export class StringSearch<T> {
  private items: Item<T>[] = [];
  private mainWords: MainWords;
  private recent?: RecentCache;

  constructor(mainWords: MainWords = "last", recent?: RecentCache) {
    this.mainWords = mainWords;
    this.recent = recent;
  }

  /** `weight`: higher wins among equally good matches. */
  add(str: string, data: T, weight = 0) {
    const { words, groups } = extractNormalizedWords(str);
    let mainGroup = 0;
    if (groups.length) {
      if (this.mainWords === "last") mainGroup = groups[groups.length - 1];
      else if (this.mainWords === "all") groups.fill(0);
    }
    let mainGroupLength = 0;
    words.forEach((w, i) => groups[i] === mainGroup && (mainGroupLength += w.length));
    this.items.push({
      data,
      words,
      groups,
      mainGroup,
      mainGroupLength,
      totalLength: str.length,
      weight,
      recentTime: this.recent?.get(str) ?? -1,
      deprecated: str.includes("eprecated"),
    });
  }

  /** Matching items, best first, with their scores. */
  queryScored(query: string): { data: T; score: number }[] {
    const { words: queryWords } = extractNormalizedWords(query);
    const byScore = new Map<number, number[]>();
    this.items.forEach((item, i) => {
      const s = scoreQueryAgainstWords(queryWords, item);
      if (s === null) return;
      const list = byScore.get(s);
      list ? list.push(i) : byScore.set(s, [i]);
    });
    const scores = [...byScore.keys()].sort((a, b) => b - a);
    const out: { data: T; score: number }[] = [];
    for (const score of scores) {
      let indices = byScore.get(score)!;
      if (score === scores[0]) {
        if (query) {
          // Shorter items are more likely the one you mean; exact matches end up on top.
          indices.sort((a, b) => {
            const A = this.items[a], B = this.items[b];
            return A.mainGroupLength - B.mainGroupLength || A.totalLength - B.totalLength;
          });
          indices = stableSort(indices, (a, b) => this.items[b].weight - this.items[a].weight);
        }
        // With a short query you probably want something you picked before.
        if (query.length <= 1) indices = stableSort(indices, (a, b) => this.items[b].recentTime - this.items[a].recentTime);
      }
      for (const i of indices) out.push({ data: this.items[i].data, score });
    }
    return out;
  }

  query(query: string): T[] {
    return this.queryScored(query).map((r) => r.data);
  }
}

function stableSort<T>(arr: T[], cmp: (a: T, b: T) => number): T[] {
  return arr
    .map((v, i) => [v, i] as const)
    .sort((a, b) => cmp(a[0], b[0]) || a[1] - b[1])
    .map(([v]) => v);
}

/**
 * Remembers picks in localStorage (Blender keeps them in recent_searches.txt). Every access
 * is guarded: private windows and blocked storage just mean "no recent picks".
 */
export function localRecentCache(key: string, max = 50) {
  const read = (): string[] => {
    try {
      const v = JSON.parse(localStorage.getItem(key) ?? "[]");
      return Array.isArray(v) ? v.filter((s) => typeof s === "string") : [];
    } catch {
      return [];
    }
  };
  let times: Map<string, number> | null = null;
  return {
    /** Larger is more recent. */
    get(str: string) {
      if (!times) {
        const list = read();
        times = new Map(list.map((s, i) => [s, list.length - i]));
      }
      return times.get(str);
    },
    list: read,
    remember(str: string) {
      times = null;
      try {
        const list = [str, ...read().filter((s) => s !== str)].slice(0, max);
        localStorage.setItem(key, JSON.stringify(list));
      } catch {
        /* storage unavailable */
      }
    },
  };
}
