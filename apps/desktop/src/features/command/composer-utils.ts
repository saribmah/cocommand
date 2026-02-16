import type {
  ExtensionPartInput,
  FilePartInput,
  MessagePartInput,
  SourcePart,
  TextPartInput,
  ToolPart,
} from "./command.types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ToolCardState = "pending" | "running" | "success" | "error";
export type FilterTab = "recent" | "extensions" | "commands" | "applications" | `ext:${string}`;
export type ComposerTagSegment =
  | { type: "text"; key: string; text: string }
  | { type: "extension"; key: string; part: ExtensionPartInput; start: number; end: number }
  | { type: "file"; key: string; part: FilePartInput; start: number; end: number };

// ---------------------------------------------------------------------------
// Composer part manipulation
// ---------------------------------------------------------------------------

export function emptyTextPart(): TextPartInput {
  return { type: "text", text: "" };
}

export function normalizeComposerParts(parts: MessagePartInput[]): MessagePartInput[] {
  const merged: MessagePartInput[] = [];
  for (const part of parts) {
    const previous = merged[merged.length - 1];
    if (part.type === "text" && previous?.type === "text") {
      previous.text += part.text;
      continue;
    }
    merged.push(part);
  }

  const cleaned = merged.filter((part, index) => {
    if (part.type !== "text") return true;
    const isLast = index === merged.length - 1;
    return isLast || part.text.length > 0;
  });

  if (cleaned.length === 0 || cleaned[cleaned.length - 1]?.type !== "text") {
    cleaned.push(emptyTextPart());
  }
  return cleaned;
}

export function defaultSourceValue(part: ExtensionPartInput | FilePartInput): string {
  if (part.type === "extension") return `@${part.extensionId}`;
  return `#${part.name}`;
}

export function resolveComposerSources(parts: MessagePartInput[]): MessagePartInput[] {
  const resolved: MessagePartInput[] = [];
  let cursor = 0;

  for (const part of parts) {
    if (part.type === "text") {
      cursor += part.text.length;
      resolved.push(part);
      continue;
    }

    const value = part.source?.value ?? defaultSourceValue(part);
    const source = {
      value,
      start: cursor,
      end: cursor + value.length,
    };
    cursor = source.end;
    resolved.push({ ...part, source });
  }

  return resolved;
}

export function commitComposerParts(parts: MessagePartInput[]): MessagePartInput[] {
  return resolveComposerSources(normalizeComposerParts(parts));
}

export function getActiveTextPartIndex(parts: MessagePartInput[]): number {
  for (let index = parts.length - 1; index >= 0; index -= 1) {
    if (parts[index]?.type === "text") return index;
  }
  return -1;
}

export function getActiveText(parts: MessagePartInput[]): string {
  const activeIndex = getActiveTextPartIndex(parts);
  if (activeIndex < 0) return "";
  const part = parts[activeIndex];
  return part?.type === "text" ? part.text : "";
}

export function updateActiveText(parts: MessagePartInput[], text: string): MessagePartInput[] {
  const next = [...parts];
  const activeIndex = getActiveTextPartIndex(next);
  if (activeIndex < 0) {
    next.push({ type: "text", text });
    return next;
  }
  const current = next[activeIndex];
  if (current?.type !== "text") return next;
  next[activeIndex] = { ...current, text };
  return next;
}

export function insertPartAfterActiveText(
  parts: MessagePartInput[],
  part: ExtensionPartInput | FilePartInput
): MessagePartInput[] {
  const next = [...parts];
  const activeIndex = getActiveTextPartIndex(next);
  if (activeIndex < 0) {
    next.push(emptyTextPart(), part, emptyTextPart());
    return next;
  }
  next.splice(activeIndex + 1, 0, part, emptyTextPart());
  return next;
}

export function removeTaggedPartBySource(
  parts: MessagePartInput[],
  match: { type: "extension" | "file"; start: number; end: number }
): MessagePartInput[] {
  const resolved = resolveComposerSources(parts);
  const index = resolved.findIndex((part) => {
    if (part.type !== match.type) return false;
    if (!part.source) return false;
    return part.source.start === match.start && part.source.end === match.end;
  });
  if (index < 0) return parts;
  const next = [...parts];
  next.splice(index, 1);
  return next;
}

export function buildTagSegments(parts: MessagePartInput[]): ComposerTagSegment[] {
  const resolved = resolveComposerSources(parts);
  const composedText = resolved
    .map((part) => {
      if (part.type === "text") return part.text;
      return part.source?.value ?? defaultSourceValue(part);
    })
    .join("");

  const ranges = resolved
    .filter((part): part is ExtensionPartInput | FilePartInput => part.type !== "text")
    .map((part) => {
      const source = part.source;
      return source
        ? {
            type: part.type,
            start: source.start,
            end: source.end,
            part,
          }
        : null;
    })
    .filter((value): value is { type: "extension" | "file"; start: number; end: number; part: ExtensionPartInput | FilePartInput } => value !== null)
    .sort((left, right) => left.start - right.start);

  const segments: ComposerTagSegment[] = [];
  let cursor = 0;
  for (const range of ranges) {
    if (range.start > cursor) {
      const text = composedText.slice(cursor, range.start);
      if (text.length > 0) {
        segments.push({
          type: "text",
          key: `text-${cursor}`,
          text,
        });
      }
    }

    if (range.type === "extension" && range.part.type === "extension") {
      segments.push({
        type: "extension",
        key: `ext-${range.start}-${range.end}-${range.part.extensionId}`,
        part: range.part,
        start: range.start,
        end: range.end,
      });
    }
    if (range.type === "file" && range.part.type === "file") {
      segments.push({
        type: "file",
        key: `file-${range.start}-${range.end}-${range.part.path}`,
        part: range.part,
        start: range.start,
        end: range.end,
      });
    }

    cursor = range.end;
  }

  if (cursor < composedText.length) {
    const text = composedText.slice(cursor);
    if (text.length > 0) {
      segments.push({
        type: "text",
        key: `text-${cursor}-tail`,
        text,
      });
    }
  }

  return segments;
}

// ---------------------------------------------------------------------------
// Sigil detection
// ---------------------------------------------------------------------------

export function getMentionState(text: string): { query: string; start: number } | null {
  const match = /(^|\s)@([^\s@]*)$/.exec(text);
  if (!match) return null;
  const start = match.index + match[1].length;
  return { query: match[2], start };
}

export function getSlashState(text: string): { query: string; start: number } | null {
  const match = /(^|\s)\/([^\s/]*)$/.exec(text);
  if (!match) return null;
  const start = match.index + match[1].length;
  return { query: match[2], start };
}

export function getHashState(text: string): { query: string; start: number } | null {
  const match = /(^|\s)#(.*)$/.exec(text);
  if (!match) return null;
  const start = match.index + match[1].length;
  return { query: match[2], start };
}

export function getStarState(text: string): { query: string; start: number } | null {
  const match = /(^|\s)\*(.*)$/.exec(text);
  if (!match) return null;
  const start = match.index + match[1].length;
  return { query: match[2], start };
}

export function findExactMentionExtensionId(
  text: string,
  extensions: { id: string; name: string }[]
): string | null {
  const trimmed = text.trim();
  if (!trimmed.startsWith("@")) return null;
  const mention = trimmed.slice(1).trim();
  const normalized = mention.toLowerCase();
  const match = extensions.find(
    (extension) =>
      extension.id.toLowerCase() === normalized ||
      extension.name.toLowerCase() === normalized
  );
  return match ? match.id : null;
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

export function normalizeQuery(value: string): string {
  return value.trim().toLowerCase();
}

export function subsequenceScore(query: string, target: string): number {
  if (!query) return 0;
  let score = 0;
  let ti = 0;
  for (let qi = 0; qi < query.length; qi += 1) {
    const q = query[qi];
    const found = target.indexOf(q, ti);
    if (found === -1) return -1;
    score += found === ti ? 2 : 1;
    ti = found + 1;
  }
  return score;
}

export function matchScore(query: string, name: string, id: string, kind: string): number {
  if (!query) return 0;
  const nameLower = name.toLowerCase();
  const idLower = id.toLowerCase();
  const kindLower = kind.toLowerCase();
  if (nameLower.includes(query) || idLower.includes(query) || kindLower.includes(query)) {
    return 100 + query.length;
  }
  const compactQuery = query.replace(/\s+/g, "");
  const nameScore = subsequenceScore(compactQuery, nameLower.replace(/\s+/g, ""));
  const idScore = subsequenceScore(compactQuery, idLower.replace(/\s+/g, ""));
  const kindScore = subsequenceScore(compactQuery, kindLower.replace(/\s+/g, ""));
  const best = Math.max(nameScore, idScore, kindScore);
  return best > 0 ? best : -1;
}

export function removeTrailingSigilQuery(
  text: string,
  sigilState: { start: number } | null
): string {
  if (sigilState) {
    return text.slice(0, sigilState.start);
  }
  return text;
}

// ---------------------------------------------------------------------------
// Response rendering helpers
// ---------------------------------------------------------------------------

export function formatPayload(value: unknown): string | undefined {
  if (value === undefined) return undefined;
  if (value === null) return "null";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function formatSourceBody(part: SourcePart): string {
  const lines = [part.title, part.url, part.filename].filter(Boolean) as string[];
  return lines.length > 0 ? lines.join("\n") : part.sourceType;
}

export function mapToolStateToCard(state: ToolPart["state"]): ToolCardState {
  switch (state.status) {
    case "pending":
      return "pending";
    case "running":
      return "running";
    case "completed":
      return "success";
    case "error":
      return "error";
    default:
      return "pending";
  }
}

export function getToolParams(state: ToolPart["state"]): string | undefined {
  return formatPayload(state.input);
}

export function getToolResult(state: ToolPart["state"]): string | undefined {
  if (state.status !== "completed") return undefined;
  return state.output;
}

export function getToolError(state: ToolPart["state"]): string | undefined {
  if (state.status !== "error") return undefined;
  return state.error;
}

export function formatFileType(mediaType?: string | null): string | undefined {
  if (!mediaType) return undefined;
  const bits = mediaType.split("/");
  if (bits.length < 2) return mediaType.toUpperCase();
  return bits[1].toUpperCase();
}
