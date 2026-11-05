// Turns an MDX body into plain text for the search index.

export function mdxToText(body: string): string {
  // Fenced code first, so code lines aren't mistaken for MDX imports. Short blocks are
  // kept: commands are worth finding.
  const codes: string[] = [];
  const withoutCode = body.replace(/```[^\n]*\n([\s\S]*?)```/g, (_, code: string) => {
    codes.push(code.length < 240 ? code.replace(/[<>{}]/g, " ") : "");
    return ` @@code${codes.length - 1}@@ `;
  });
  return (
    withoutCode
      // MDX imports / exports
      .replace(/^\s*(import|export)\s.*$/gm, "")
      // JSX comments and expressions
      .replace(/\{\/\*[\s\S]*?\*\/\}/g, " ")
      // keep useful attribute text (title="..." label="..." caption="...")
      .replace(/<[A-Z][\w.]*\b([^>]*)>/g, (_, attrs: string) => {
        const kept = [...attrs.matchAll(/(?:title|label|caption|description)="([^"]*)"/g)].map((m) => m[1]);
        return ` ${kept.join(" ")} `;
      })
      .replace(/<\/?[A-Za-z][^>]*>/g, " ")
      // markdown syntax
      .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/^#{1,6}\s+/gm, "")
      .replace(/^\s*[-*+]\s+/gm, "")
      .replace(/^\s*\|?[-:| ]+\|?\s*$/gm, " ")
      .replace(/[*_`>|]/g, " ")
      .replace(/@@code(\d+)@@/g, (_, i: string) => codes[Number(i)])
      .replace(/\s+/g, " ")
      .trim()
  );
}

export interface Chunk {
  /** Heading text, or "" for the part before the first heading. */
  heading: string;
  text: string;
}

/** Splits a body at its ## and ### headings. */
export function splitByHeadings(body: string): Chunk[] {
  const lines = body.replace(/```[\s\S]*?```/g, (m) => m.replace(/^#/gm, "​#")).split("\n");
  const chunks: Chunk[] = [{ heading: "", text: "" }];
  for (const line of lines) {
    const m = /^(#{2,3})\s+(.+?)\s*$/.exec(line);
    if (m) chunks.push({ heading: m[2], text: "" });
    else chunks[chunks.length - 1].text += line + "\n";
  }
  return chunks.map((c) => ({ heading: c.heading, text: mdxToText(c.text.replace(/​/g, "")) }));
}
