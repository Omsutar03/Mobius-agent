/**
 * Strips tool code blocks (e.g. ```web_search...``` or ```bash...```) from the text
 * so raw tool invocation code isn't rendered in the GUI chat view.
 */
export function stripToolBlocks(text: string): string {
  if (!text) return "";

  // 1. Strip complete tool blocks
  const completeToolRegex = /```(?:bash|sh|read|write|edit|web_search|read_webpage)[\s\S]*?```/gi;

  // 2. Strip unclosed tool blocks currently being actively streamed
  const unclosedToolRegex = /```(?:bash|sh|read|write|edit|web_search|read_webpage)[\s\S]*$/gi;

  return text
    .replace(completeToolRegex, "")
    .replace(unclosedToolRegex, "")
    .trim();
}
