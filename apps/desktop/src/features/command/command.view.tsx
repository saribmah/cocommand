import "@cocommand/ui";
import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { hideWindow, openSettingsWindow } from "../../lib/ipc";
import {
  ActionHint,
  ArrowIcon,
  Badge,
  Chip,
  ChipGroup,
  CloseButton,
  CommandIcon,
  CommandPaletteShell,
  ContentArea,
  Divider,
  ErrorCard,
  ExtensionIcon,
  FileCard,
  FilterArea,
  FooterArea,
  HeaderArea,
  HintBar,
  HintItem,
  Icon,
  IconContainer,
  KeyHint,
  ListItem,
  ListSection,
  MarkdownResponseCard,
  ReasoningCard,
  ResponseStack,
  SearchIcon,
  SearchField,
  StatusBadge,
  Text,
  ToolCallCard,
} from "@cocommand/ui";
import { useApplicationContext } from "../application/application.context";
import type { ApplicationInfo } from "../application/application.types";
import { useExtensionContext } from "../extension/extension.context";
import { useSessionContext } from "../session/session.context";
import { useServerContext } from "../server/server.context";
import { useCommandContext } from "./command.context";
import type {
  ExtensionPartInput,
  FilePartInput,
  MessagePartInput,
  SourcePart,
  TextPartInput,
  ToolPart,
} from "./command.types";
import { hasExtensionView } from "../extension/extension-views";
import { ExtensionViewContainer } from "./components/ExtensionViewContainer";
import styles from "./command.module.css";

const FileIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
    <polyline points="14,2 14,8 20,8" />
  </svg>
);

const FolderIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" />
  </svg>
);

const RemoveIcon = (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M18 6L6 18" />
    <path d="M6 6L18 18" />
  </svg>
);

const ApplicationIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="3" y="4" width="18" height="14" rx="2" />
    <path d="M8 20h8" />
    <path d="M12 18v2" />
  </svg>
);

type ToolCardState = "pending" | "running" | "success" | "error";
type FilterTab = "recent" | "extensions" | "commands" | "applications" | `ext:${string}`;
type ComposerTagSegment =
  | { type: "text"; key: string; text: string }
  | { type: "extension"; key: string; part: ExtensionPartInput; start: number; end: number }
  | { type: "file"; key: string; part: FilePartInput; start: number; end: number };

function emptyTextPart(): TextPartInput {
  return { type: "text", text: "" };
}

function normalizeComposerParts(parts: MessagePartInput[]): MessagePartInput[] {
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

function defaultSourceValue(part: ExtensionPartInput | FilePartInput): string {
  if (part.type === "extension") return `@${part.extensionId}`;
  return `#${part.name}`;
}

function resolveComposerSources(parts: MessagePartInput[]): MessagePartInput[] {
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

function commitComposerParts(parts: MessagePartInput[]): MessagePartInput[] {
  return resolveComposerSources(normalizeComposerParts(parts));
}

function getActiveTextPartIndex(parts: MessagePartInput[]): number {
  for (let index = parts.length - 1; index >= 0; index -= 1) {
    if (parts[index]?.type === "text") return index;
  }
  return -1;
}

function getActiveText(parts: MessagePartInput[]): string {
  const activeIndex = getActiveTextPartIndex(parts);
  if (activeIndex < 0) return "";
  const part = parts[activeIndex];
  return part?.type === "text" ? part.text : "";
}

function updateActiveText(parts: MessagePartInput[], text: string): MessagePartInput[] {
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

function insertPartAfterActiveText(
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

function removeTaggedPartBySource(
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

function buildTagSegments(parts: MessagePartInput[]): ComposerTagSegment[] {
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

function getMentionState(text: string): { query: string; start: number } | null {
  const match = /(^|\s)@([^\s@]*)$/.exec(text);
  if (!match) return null;
  const start = match.index + match[1].length;
  return { query: match[2], start };
}

function getSlashState(text: string): { query: string; start: number } | null {
  const match = /(^|\s)\/([^\s/]*)$/.exec(text);
  if (!match) return null;
  const start = match.index + match[1].length;
  return { query: match[2], start };
}

function getHashState(text: string): { query: string; start: number } | null {
  const match = /(^|\s)#(.*)$/.exec(text);
  if (!match) return null;
  const start = match.index + match[1].length;
  return { query: match[2], start };
}

function getStarState(text: string): { query: string; start: number } | null {
  const match = /(^|\s)\*(.*)$/.exec(text);
  if (!match) return null;
  const start = match.index + match[1].length;
  return { query: match[2], start };
}

function findExactMentionExtensionId(
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

function normalizeQuery(value: string): string {
  return value.trim().toLowerCase();
}

function subsequenceScore(query: string, target: string): number {
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

function matchScore(query: string, name: string, id: string, kind: string): number {
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

function formatPayload(value: unknown): string | undefined {
  if (value === undefined) return undefined;
  if (value === null) return "null";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function formatSourceBody(part: SourcePart): string {
  const lines = [part.title, part.url, part.filename].filter(Boolean) as string[];
  return lines.length > 0 ? lines.join("\n") : part.sourceType;
}

function mapToolStateToCard(state: ToolPart["state"]): ToolCardState {
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

function getToolParams(state: ToolPart["state"]): string | undefined {
  return formatPayload(state.input);
}

function getToolResult(state: ToolPart["state"]): string | undefined {
  if (state.status !== "completed") return undefined;
  return state.output;
}

function getToolError(state: ToolPart["state"]): string | undefined {
  if (state.status !== "error") return undefined;
  return state.error;
}

function formatFileType(mediaType?: string | null): string | undefined {
  if (!mediaType) return undefined;
  const bits = mediaType.split("/");
  if (bits.length < 2) return mediaType.toUpperCase();
  return bits[1].toUpperCase();
}

function removeTrailingSigilQuery(
  text: string,
  sigilState: { start: number } | null
): string {
  if (sigilState) {
    return text.slice(0, sigilState.start);
  }
  return text;
}

export function CommandView() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const inputId = useId();
  const [activeTab, setActiveTab] = useState<FilterTab>("recent");
  const [mentionIndex, setMentionIndex] = useState(0);
  const [slashIndex, setSlashIndex] = useState(0);
  const [applicationIndex, setApplicationIndex] = useState(0);

  const draftParts = useCommandContext((state) => state.draftParts);
  const setDraftParts = useCommandContext((state) => state.setDraftParts);
  const isSubmitting = useCommandContext((state) => state.isSubmitting);
  const parts = useCommandContext((state) => state.parts);
  const error = useCommandContext((state) => state.error);
  const setError = useCommandContext((state) => state.setError);
  const submit = useCommandContext((state) => state.submit);
  const dismiss = useCommandContext((state) => state.dismiss);
  const reset = useCommandContext((state) => state.reset);
  const sendMessage = useSessionContext((state) => state.sendMessage);
  const serverInfo = useServerContext((state) => state.info);
  const extensions = useExtensionContext((state) => state.extensions);
  const extensionsLoaded = useExtensionContext((state) => state.isLoaded);
  const fetchExtensions = useExtensionContext((state) => state.fetchExtensions);
  const openExtension = useExtensionContext((state) => state.openExtension);

  const applications = useApplicationContext((state) => state.applications);
  const applicationsCount = useApplicationContext((state) => state.count);
  const applicationsLoaded = useApplicationContext((state) => state.isLoaded);
  const applicationsLoading = useApplicationContext((state) => state.isLoading);
  const applicationsError = useApplicationContext((state) => state.error);
  const fetchApplications = useApplicationContext((state) => state.fetchApplications);
  const openApplication = useApplicationContext((state) => state.openApplication);
  const clearApplications = useApplicationContext((state) => state.clear);

  const composerParts = useMemo(
    () => commitComposerParts(draftParts),
    [draftParts]
  );
  const activeTextIndex = useMemo(
    () => getActiveTextPartIndex(composerParts),
    [composerParts]
  );
  const activeText = useMemo(() => getActiveText(composerParts), [composerParts]);
  const committedParts = useMemo(() => {
    if (activeTextIndex < 0) return composerParts;
    return composerParts.slice(0, activeTextIndex);
  }, [activeTextIndex, composerParts]);
  const tagSegments = useMemo(
    () => buildTagSegments(committedParts),
    [committedParts]
  );

  const extensionPills = useMemo(
    () =>
      composerParts
        .filter(
          (p): p is ExtensionPartInput =>
            p.type === "extension" && hasExtensionView(p.extensionId)
        )
        .map((p) => ({ extensionId: p.extensionId, name: p.name })),
    [composerParts]
  );

  const mentionState = useMemo(() => getMentionState(activeText), [activeText]);
  const slashState = useMemo(() => getSlashState(activeText), [activeText]);
  const hashState = useMemo(() => getHashState(activeText), [activeText]);
  const starState = useMemo(() => getStarState(activeText), [activeText]);
  const slashCommands = useMemo(
    () => [{ id: "settings", name: "Settings", description: "Open the settings window" }],
    []
  );

  const applyComposerParts = (next: MessagePartInput[]) => {
    setDraftParts(commitComposerParts(next));
  };

  const updateComposerText = (value: string) => {
    applyComposerParts(updateActiveText(composerParts, value));
  };

  useEffect(() => {
    if (!serverInfo) return;
    fetchExtensions();
  }, [serverInfo, fetchExtensions]);

  useEffect(() => {
    if (!mentionState) return;
    if (extensionsLoaded) return;
    fetchExtensions();
  }, [mentionState, extensionsLoaded, fetchExtensions]);

  useEffect(() => {
    clearApplications();
  }, [serverInfo?.addr, clearApplications]);

  useEffect(() => {
    const node = document.getElementById(inputId) as HTMLInputElement | null;
    node?.focus();
  }, [inputId, parts]);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node) return;
    requestAnimationFrame(() => {
      node.scrollTop = node.scrollHeight;
    });
  }, [parts]);

  const filteredExtensions = useMemo(() => {
    const query = mentionState
      ? normalizeQuery(mentionState.query)
      : activeTab === "extensions"
      ? normalizeQuery(activeText)
      : "";
    if (!mentionState && activeTab !== "extensions") return [];
    if (!mentionState) {
      if (!query) {
        return [...extensions].sort((a, b) => a.name.localeCompare(b.name));
      }
      const ranked = extensions
        .map((extension) => ({
          extension,
          score: matchScore(query, extension.name, extension.id, extension.kind),
        }))
        .filter((entry) => entry.score >= 0)
        .sort((a, b) => b.score - a.score);
      return ranked.slice(0, 8).map((entry) => entry.extension);
    }
    const ranked = extensions
      .map((extension) => ({
        extension,
        score: matchScore(query, extension.name, extension.id, extension.kind),
      }))
      .filter((entry) => (query.length === 0 ? true : entry.score >= 0))
      .sort((a, b) => b.score - a.score);
    return ranked.slice(0, 8).map((entry) => entry.extension);
  }, [activeTab, activeText, extensions, mentionState]);

  useEffect(() => {
    if (mentionState || activeTab === "extensions") {
      setMentionIndex(0);
    }
  }, [activeTab, mentionState?.query, mentionState?.start]);

  useEffect(() => {
    if ((!mentionState && slashState) || activeTab === "commands") {
      setSlashIndex(0);
    }
  }, [activeTab, mentionState, slashState?.query, slashState?.start]);

  const filteredSlashCommands = useMemo(() => {
    if (!slashState && activeTab !== "commands") return [];
    const query = slashState
      ? normalizeQuery(slashState.query)
      : activeTab === "commands"
      ? normalizeQuery(activeText)
      : "";
    if (!query) return slashCommands;
    const ranked = slashCommands
      .map((command) => ({
        command,
        score: matchScore(query, command.name, command.id, command.description),
      }))
      .filter((entry) => (query.length === 0 ? true : entry.score >= 0))
      .sort((a, b) => b.score - a.score);
    return ranked.slice(0, 6).map((entry) => entry.command);
  }, [activeTab, activeText, slashCommands, slashState]);

  const filteredApplications = useMemo(() => {
    if (!starState && activeTab !== "applications") return [];
    const query = starState
      ? normalizeQuery(starState.query)
      : activeTab === "applications"
      ? normalizeQuery(activeText)
      : "";
    if (!query) {
      return [...applications].sort((a, b) => a.name.localeCompare(b.name));
    }
    const ranked = applications
      .map((application) => ({
        application,
        score: matchScore(query, application.name, application.id, application.path),
      }))
      .filter((entry) => entry.score >= 0)
      .sort((a, b) => b.score - a.score);
    return ranked.slice(0, 20).map((entry) => entry.application);
  }, [activeTab, activeText, applications, starState]);

  const showExtensionView = activeTab.startsWith("ext:");
  const activeExtensionId = showExtensionView ? activeTab.slice(4) : null;
  const showExtensionsList = !showExtensionView && (activeTab === "extensions" || !!mentionState);
  const showCommandsList = !showExtensionView && !showExtensionsList && (activeTab === "commands" || !!slashState);
  const showApplicationsList =
    !showExtensionView &&
    !showExtensionsList &&
    !showCommandsList &&
    (activeTab === "applications" || !!starState);

  useEffect(() => {
    if (!showApplicationsList) return;
    setApplicationIndex(0);
  }, [showApplicationsList, starState?.query, starState?.start, activeTab]);

  useEffect(() => {
    if (!showApplicationsList) return;
    if (applicationsLoaded || applicationsLoading) return;
    fetchApplications().catch(() => {
      // application store already tracks error state
    });
  }, [
    fetchApplications,
    applicationsLoaded,
    applicationsLoading,
    showApplicationsList,
  ]);

  useEffect(() => {
    if (!hashState) return;
    if (activeTab === `ext:filesystem`) return;
    const alreadyHasFilesystem = composerParts.some(
      (p) => p.type === "extension" && p.extensionId === "filesystem"
    );
    if (!alreadyHasFilesystem) {
      let nextParts = updateActiveText(composerParts, removeTrailingSigilQuery(activeText, hashState));
      nextParts = insertPartAfterActiveText(nextParts, {
        type: "extension",
        extensionId: "filesystem",
        name: "Files",
        kind: "builtin",
        source: { value: "@filesystem", start: 0, end: 0 },
      });
      applyComposerParts(nextParts);
    }
    setActiveTab(`ext:filesystem`);
  }, [hashState]);

  const focusInput = () => {
    requestAnimationFrame(() => {
      inputRef.current?.focus();
    });
  };

  const executeSlashCommand = (id: string) => {
    if (id !== "settings") return;
    openSettingsWindow()
      .then(() => {
        reset();
        hideWindow();
      })
      .catch((err) => {
        setError(String(err));
      });
  };

  const openApplicationById = (application: ApplicationInfo) => {
    openApplication({ id: application.id })
      .then(() => {
        const nextText = removeTrailingSigilQuery(activeText, starState);
        updateComposerText(nextText);
        setActiveTab("recent");
        focusInput();
      })
      .catch((err) => {
        setError(String(err));
      });
  };

  const selectExtension = (extension: { id: string; name: string; kind: string }) => {
    const nextText = removeTrailingSigilQuery(activeText, mentionState);
    let nextParts = updateActiveText(composerParts, nextText);
    nextParts = insertPartAfterActiveText(nextParts, {
      type: "extension",
      extensionId: extension.id,
      name: extension.name,
      kind: extension.kind,
      source: {
        value: `@${extension.id}`,
        start: 0,
        end: 0,
      },
    });
    applyComposerParts(nextParts);
    if (hasExtensionView(extension.id)) {
      setActiveTab(`ext:${extension.id}`);
    } else {
      setActiveTab("recent");
    }
    focusInput();
  };

  const selectFile = (entry: {
    path: string;
    name: string;
    type: "file" | "directory" | "symlink" | "other";
  }) => {
    const normalizedName =
      entry.name.trim().length > 0
        ? entry.name
        : entry.path.split("/").filter(Boolean).pop() ?? entry.path;

    const nextText = removeTrailingSigilQuery(activeText, hashState);
    let nextParts = updateActiveText(composerParts, nextText);
    nextParts = insertPartAfterActiveText(nextParts, {
      type: "file",
      path: entry.path,
      name: normalizedName,
      entryType: entry.type,
      source: {
        value: `#${normalizedName}`,
        start: 0,
        end: 0,
      },
    });
    applyComposerParts(nextParts);
    setActiveTab("recent");
    focusInput();
  };

  const removeTaggedSegment = (segment: ComposerTagSegment) => {
    if (segment.type !== "extension" && segment.type !== "file") return;
    const next = removeTaggedPartBySource(composerParts, {
      type: segment.type,
      start: segment.start,
      end: segment.end,
    });
    applyComposerParts(next);
    focusInput();
  };

  const insertSigilAtCursor = (sigil: "@" | "/" | "#" | "*") => {
    const node = inputRef.current;
    const start = node?.selectionStart ?? activeText.length;
    const end = node?.selectionEnd ?? activeText.length;
    let replaceStart = start;
    let replaceEnd = end;

    if (start === end) {
      const prevChar = start > 0 ? activeText[start - 1] : "";
      const nextChar = start < activeText.length ? activeText[start] : "";
      if (prevChar === "@" || prevChar === "/" || prevChar === "#" || prevChar === "*") {
        replaceStart = start - 1;
        replaceEnd = start;
      } else if (nextChar === "@" || nextChar === "/" || nextChar === "#" || nextChar === "*") {
        replaceStart = start;
        replaceEnd = start + 1;
      }
    }

    const nextValue = `${activeText.slice(0, replaceStart)}${sigil}${activeText.slice(replaceEnd)}`;
    const caret = replaceStart + sigil.length;
    updateComposerText(nextValue);
    requestAnimationFrame(() => {
      const current = inputRef.current;
      if (!current) return;
      current.focus();
      current.setSelectionRange(caret, caret);
    });
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (
      e.key === "Backspace" &&
      activeText.length === 0 &&
      (activeTab === "extensions" ||
        activeTab === "commands" ||
        activeTab === "applications" ||
        activeTab.startsWith("ext:"))
    ) {
      e.preventDefault();
      setActiveTab("recent");
      return;
    }

    if (e.key === "Backspace" && activeText.length === 0) {
      const activeIndex = getActiveTextPartIndex(composerParts);
      if (activeIndex > 0) {
        const previous = composerParts[activeIndex - 1];
        e.preventDefault();
        if (previous?.type === "text") {
          const next = [...composerParts];
          next.splice(activeIndex - 1, 2, { type: "text", text: previous.text });
          applyComposerParts(next);
        } else {
          const next = [...composerParts];
          next.splice(activeIndex - 1, 1);
          applyComposerParts(next);
        }
        return;
      }
    }

    if (showExtensionView) {
      if (e.key === "Escape") {
        e.preventDefault();
        setActiveTab("recent");
        return;
      }
      return;
    }

    if (showExtensionsList && filteredExtensions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setMentionIndex((mentionIndex + 1) % filteredExtensions.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setMentionIndex(
          mentionIndex <= 0 ? filteredExtensions.length - 1 : mentionIndex - 1
        );
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const selected = filteredExtensions[mentionIndex];
        if (selected) {
          selectExtension({
            id: selected.id,
            name: selected.name,
            kind: selected.kind,
          });
        }
        return;
      }
    }

    if (showCommandsList && filteredSlashCommands.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSlashIndex((idx) => (idx + 1) % filteredSlashCommands.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSlashIndex((idx) =>
          idx <= 0 ? filteredSlashCommands.length - 1 : idx - 1
        );
        return;
      }
      if (e.key === "Enter") {
        const selected = filteredSlashCommands[slashIndex];
        if (selected) {
          e.preventDefault();
          executeSlashCommand(selected.id);
          return;
        }
      }
    }

    if (showApplicationsList && filteredApplications.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setApplicationIndex((idx) => (idx + 1) % filteredApplications.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setApplicationIndex((idx) =>
          idx <= 0 ? filteredApplications.length - 1 : idx - 1
        );
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const selected = filteredApplications[applicationIndex];
        if (selected) {
          openApplicationById(selected);
        }
        return;
      }
    }

    switch (e.key) {
      case "Enter":
        e.preventDefault();
        {
          const mentionExtensionId = findExactMentionExtensionId(activeText, extensions);
          if (mentionExtensionId && committedParts.length === 0) {
            openExtension(mentionExtensionId)
              .then(() => {
                reset();
              })
              .catch((err) => {
                setError(String(err));
              });
            return;
          }
          submit(sendMessage);
        }
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  };

  const inputTargetTags =
    tagSegments.length > 0 ? (
      <div className={styles.targetTagRow}>
        {tagSegments.map((segment) => {
          if (segment.type === "text") {
            return (
              <span key={segment.key} className={styles.targetTextChunk}>
                {segment.text}
              </span>
            );
          }
          if (segment.type === "extension") {
            return (
              <span
                key={segment.key}
                className={styles.targetTag}
                title={`${segment.part.kind ?? "extension"} extension`}
              >
                <Icon size={14}>{ExtensionIcon}</Icon>
                <span className={styles.targetTagLabel}>@{segment.part.name}</span>
                <button
                  type="button"
                  className={styles.targetTagRemove}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => removeTaggedSegment(segment)}
                  aria-label={`Remove @${segment.part.name}`}
                >
                  <Icon size={12}>{RemoveIcon}</Icon>
                </button>
              </span>
            );
          }
          return (
            <span key={segment.key} className={styles.targetTag} title={segment.part.path}>
              <Icon size={14}>
                {segment.part.entryType === "directory" ? FolderIcon : FileIcon}
              </Icon>
              <span className={styles.targetTagLabel}>{segment.part.name}</span>
              <button
                type="button"
                className={styles.targetTagRemove}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => removeTaggedSegment(segment)}
                aria-label={`Remove ${segment.part.name}`}
              >
                <Icon size={12}>{RemoveIcon}</Icon>
              </button>
            </span>
          );
        })}
      </div>
    ) : undefined;
  const placeholder =
    activeText.length === 0 && tagSegments.length === 0
      ? "How can I help..."
      : "";

  const showResponses = parts.length > 0 || !!error;

  return (
    <main className="app-shell">
      <CommandPaletteShell className={`app-shell-panel ${styles.shell}`}>
        <HeaderArea>
          <div className={styles.headerRow}>
            <SearchField
              className={styles.searchField}
              icon={<Icon>{SearchIcon}</Icon>}
              beforeInput={inputTargetTags}
              placeholder={placeholder}
              inputRef={inputRef}
              inputProps={{
                id: inputId,
                value: activeText,
                onChange: (e) => updateComposerText(e.target.value),
                onKeyDown: handleKeyDown,
                disabled: isSubmitting,
                spellCheck: false,
                autoComplete: "off",
              }}
            />
            <StatusBadge
              status={serverInfo ? "good" : "warn"}
              label={serverInfo ? "online" : "offline"}
            />
          </div>
          <Divider />
        </HeaderArea>

        <FilterArea>
          <div className={styles.filterRow}>
            <ChipGroup>
              <Chip
                label="Recent"
                active={
                  activeTab === "recent" &&
                  !mentionState &&
                  !slashState &&
                  !hashState &&
                  !starState
                }
                onClick={() => setActiveTab("recent")}
              />
              <Chip
                label="Extensions"
                active={activeTab === "extensions" || !!mentionState}
                onClick={() => {
                  setActiveTab("extensions");
                  fetchExtensions();
                  insertSigilAtCursor("@");
                }}
              />
              <Chip
                label="Commands"
                active={activeTab === "commands" || (!!slashState && !mentionState)}
                onClick={() => {
                  setActiveTab("commands");
                  insertSigilAtCursor("/");
                }}
              />
              <Chip
                label="Applications"
                active={activeTab === "applications" || !!starState}
                onClick={() => {
                  setActiveTab("applications");
                  fetchApplications().catch(() => {
                    // application store already tracks error state
                  });
                  insertSigilAtCursor("*");
                }}
              />
              {extensionPills.map((pill) => (
                <Chip
                  key={`ext-pill-${pill.extensionId}`}
                  label={pill.name}
                  active={activeTab === `ext:${pill.extensionId}`}
                  onClick={() => setActiveTab(`ext:${pill.extensionId}`)}
                />
              ))}
            </ChipGroup>
            {isSubmitting ? <Badge>Working...</Badge> : null}
          </div>
        </FilterArea>

        <ContentArea className={styles.content}>
          {showExtensionView && activeExtensionId ? (
            <ExtensionViewContainer
              extensionId={activeExtensionId}
              onSelectFile={selectFile}
            />
          ) : (
          <div className={styles.scrollArea} ref={scrollRef}>
            {showExtensionsList ? (
              <ListSection label="Extensions">
                {filteredExtensions.length > 0 ? (
                  filteredExtensions.map((extension, index) => (
                    <ListItem
                      key={extension.id}
                      title={extension.name}
                      subtitle={`${extension.kind} / ${extension.id}`}
                      icon={
                        <IconContainer>
                          <Icon>{ExtensionIcon}</Icon>
                        </IconContainer>
                      }
                      selected={index === mentionIndex}
                      onMouseDown={(event) => {
                        event.preventDefault();
                        selectExtension({
                          id: extension.id,
                          name: extension.name,
                          kind: extension.kind,
                        });
                      }}
                    />
                  ))
                ) : (
                  <Text size="sm" tone="secondary">
                    {extensionsLoaded ? "No extensions found." : "Loading extensions..."}
                  </Text>
                )}
              </ListSection>
            ) : null}

            {showCommandsList ? (
              <ListSection label="Commands">
                {filteredSlashCommands.map((command, index) => (
                  <ListItem
                    key={command.id}
                    title={`/${command.id}`}
                    subtitle={command.description}
                    icon={
                      <IconContainer>
                        <Icon>{CommandIcon}</Icon>
                      </IconContainer>
                    }
                    rightMeta={<ActionHint label="Enter" icon={<Icon>{ArrowIcon}</Icon>} />}
                    selected={index === slashIndex}
                    onMouseDown={(event) => {
                      event.preventDefault();
                      executeSlashCommand(command.id);
                    }}
                  />
                ))}
              </ListSection>
            ) : null}

            {showApplicationsList ? (
              <ListSection
                label={
                  applicationsLoading
                    ? "Loading applications..."
                    : `Applications${applicationsLoaded ? ` (${applicationsCount})` : ""}`
                }
              >
                {applicationsError ? (
                  <Text size="sm" tone="secondary">
                    {applicationsError}
                  </Text>
                ) : filteredApplications.length > 0 ? (
                  filteredApplications.map((application, index) => (
                    <ListItem
                      key={application.id}
                      title={application.name}
                      subtitle={application.bundleId ?? application.path}
                      icon={
                        <IconContainer>
                          {application.icon ? (
                            <img
                              src={application.icon}
                              alt=""
                              width={18}
                              height={18}
                              style={{ borderRadius: 4, objectFit: "contain" }}
                            />
                          ) : (
                            <Icon>{ApplicationIcon}</Icon>
                          )}
                        </IconContainer>
                      }
                      rightMeta={<ActionHint label="Open" icon={<Icon>{ArrowIcon}</Icon>} />}
                      selected={index === applicationIndex}
                      onMouseDown={(event) => {
                        event.preventDefault();
                        openApplicationById(application);
                      }}
                    />
                  ))
                ) : starState?.query ? (
                  <Text size="sm" tone="secondary">
                    {applicationsLoading ? "Loading applications..." : "No applications found."}
                  </Text>
                ) : (
                  <Text size="sm" tone="secondary">
                    Type to search applications...
                  </Text>
                )}
              </ListSection>
            ) : null}

            {(showExtensionsList || showCommandsList || showApplicationsList) && showResponses ? (
              <Divider />
            ) : null}

            {showResponses ? (
              <ResponseStack>
                {error ? <ErrorCard message={error} /> : null}
                {parts.map((part) => {
                  switch (part.type) {
                    case "text":
                      return <MarkdownResponseCard key={part.id} body={part.text} />;
                    case "reasoning":
                      return <ReasoningCard key={part.id} reasoning={part.text} />;
                    case "tool":
                      return (
                        <ToolCallCard
                          key={part.id}
                          toolName={part.tool}
                          toolId={part.callId}
                          state={mapToolStateToCard(part.state)}
                          params={getToolParams(part.state)}
                          result={getToolResult(part.state)}
                          errorMessage={getToolError(part.state)}
                        />
                      );
                    case "file":
                      return (
                        <FileCard
                          key={part.id}
                          fileName={part.name ?? "Untitled file"}
                          fileType={formatFileType(part.mediaType)}
                        />
                      );
                    case "source":
                      return (
                        <MarkdownResponseCard
                          key={part.id}
                          label="Source"
                          body={formatSourceBody(part)}
                        />
                      );
                    default:
                      return null;
                  }
                })}
              </ResponseStack>
            ) : !showExtensionsList && !showCommandsList && !showApplicationsList ? (
              <Text size="sm" tone="secondary">
                Type a command, use @ to target an extension, / for shortcuts, or * for applications.
              </Text>
            ) : null}
          </div>
          )}
        </ContentArea>

        <FooterArea>
          <HintBar
            left={
              <>
                <HintItem label="Navigate" keyHint={<KeyHint keys={["↑", "↓"]} />} />
                <HintItem label="Enter" keyHint={<KeyHint keys="↵" />} />
                <HintItem label="Extensions" keyHint={<KeyHint keys="@" />} />
                <HintItem label="Command" keyHint={<KeyHint keys="/" />} />
                <HintItem label="Applications" keyHint={<KeyHint keys="*" />} />
              </>
            }
            right={<CloseButton keyLabel="esc" onClick={dismiss} />}
          />
        </FooterArea>
      </CommandPaletteShell>
    </main>
  );
}
