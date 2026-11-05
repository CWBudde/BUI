// The static search index, built with the site: /search.json
import type { APIRoute } from "astro";
import { render } from "astro:content";
import { docHref, getDocs } from "../lib/docs";
import { splitByHeadings } from "../lib/search-text";

export interface SearchRecord {
  /** Page title */
  t: string;
  /** Section heading inside the page ("" for the page itself) */
  h: string;
  /** URL, with #anchor for headings */
  u: string;
  /** Sidebar section */
  s: string;
  /** Text (trimmed) */
  x: string;
  /** Page status, when not stable */
  st?: string;
}

export const GET: APIRoute = async () => {
  const records: SearchRecord[] = [];
  for (const doc of await getDocs()) {
    const { headings } = await render(doc);
    const anchors = headings.filter((h) => h.depth === 2 || h.depth === 3);
    const chunks = splitByHeadings(doc.body ?? "");
    const url = docHref(doc);
    const status = doc.data.status === "stable" ? undefined : doc.data.status;
    chunks.forEach((chunk, i) => {
      if (i === 0) {
        records.push({
          t: doc.data.title,
          h: "",
          u: url,
          s: doc.data.section,
          x: `${doc.data.description} ${doc.data.languages.join(" ")} ${doc.data.badges.join(" ")} ${chunk.text}`.slice(0, 600),
          st: status,
        });
        return;
      }
      const anchor = anchors[i - 1];
      records.push({
        t: doc.data.title,
        h: anchor?.text ?? chunk.heading,
        u: anchor ? `${url}#${anchor.slug}` : url,
        s: doc.data.section,
        x: chunk.text.slice(0, 600),
        st: status,
      });
    });
  }
  return new Response(JSON.stringify(records), { headers: { "Content-Type": "application/json; charset=utf-8" } });
};
