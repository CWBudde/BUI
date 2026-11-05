import { execFileSync } from "node:child_process";
import { statSync } from "node:fs";
import { getCollection, type CollectionEntry } from "astro:content";
import { sections, type SectionLabel } from "../config/nav";
import { site } from "../config/site";

export type Doc = CollectionEntry<"docs">;

/** Prefix a site-relative path with the configured base (works under a sub-path deploy). */
export function href(path = ""): string {
  const base = import.meta.env.BASE_URL.replace(/\/$/, "");
  if (/^(https?:|mailto:|#)/.test(path)) return path;
  return `${base}/${path.replace(/^\//, "")}`;
}

export const docHref = (doc: Doc) => href(`docs/${doc.id}/`);

/** A link to a file in the BUI repository, e.g. repoFile("docs/rust.md"). */
export const repoFile = (path: string) => site.blobBase + path.replace(/^\//, "");

const sectionIndex = (label: SectionLabel) => sections.findIndex((s) => s.label === label);

let cache: Doc[] | undefined;

/** Every published page, in reading order (section order, then `order`, then title). */
export async function getDocs(): Promise<Doc[]> {
  if (cache) return cache;
  const all = await getCollection("docs", (d) => !d.data.draft);
  cache = all.sort(
    (a, b) =>
      sectionIndex(a.data.section) - sectionIndex(b.data.section) ||
      a.data.order - b.data.order ||
      a.data.title.localeCompare(b.data.title),
  );
  return cache;
}

export async function getGroups() {
  const docs = await getDocs();
  return sections
    .map((section) => ({ ...section, docs: docs.filter((d) => d.data.section === section.label) }))
    .filter((g) => g.docs.length > 0);
}

export async function getNeighbours(doc: Doc) {
  const docs = await getDocs();
  const i = docs.findIndex((d) => d.id === doc.id);
  return { prev: i > 0 ? docs[i - 1] : undefined, next: i >= 0 && i < docs.length - 1 ? docs[i + 1] : undefined };
}

/** Last commit date of the source file, or its modification time when it isn't committed yet. */
export function lastUpdated(filePath: string | undefined): Date | undefined {
  if (!filePath) return undefined;
  try {
    const out = execFileSync("git", ["log", "-1", "--format=%cI", "--", filePath], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return new Date(out);
  } catch {
    /* not a git checkout: fall through */
  }
  try {
    return statSync(filePath).mtime;
  } catch {
    return undefined;
  }
}

export const statusLabel = {
  stable: "Stable",
  preview: "Preview",
  "coming-soon": "Coming soon",
} as const;
