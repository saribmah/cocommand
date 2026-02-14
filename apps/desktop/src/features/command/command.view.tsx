import "@cocommand/ui";
import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
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
import { useExtensionContext } from "../extension/extension.context";
import { useFileSystemContext } from "../filesystem/filesystem.context";
import { useSessionContext } from "../session/session.context";
import { useServerContext } from "../server/server.context";
import { useCommandBar } from "./commandbar";
import type { SourcePart, ToolPart } from "./command.types";
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

function applyMention(
  text: string,
  mention: { start: number },
  name: string
): string {
  return `${text.slice(0, mention.start)}@${name} `;
}

function applySlashCommand(
  text: string,
  slash: { start: number },
  id: string
): string {
  return `${text.slice(0, slash.start)}/${id} `;
}

function resolveMentions(
  text: string,
  extensions: { id: string; name: string }[]
): string {
  return text.replace(/@([^\s@]+)/g, (full, name) => {
    const normalized = String(name).trim().toLowerCase();
    const match = extensions.find(
      (extension) =>
        extension.name.toLowerCase() === normalized ||
        extension.id.toLowerCase() === normalized
    );
    if (!match) return full;
    return `@${match.id}`;
  });
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

type ToolCardState = "pending" | "running" | "success" | "error";
type FilterTab = "recent" | "extensions" | "commands" | "files";

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

function appendMention(text: string, name: string): string {
  const separator = text.length === 0 || text.endsWith(" ") ? "" : " ";
  return `${text}${separator}@${name} `;
}

function appendSlashCommand(text: string, id: string): string {
  const separator = text.length === 0 || text.endsWith(" ") ? "" : " ";
  return `${text}${separator}/${id} `;
}

export function CommandView() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const inputId = useId();
  const [activeTab, setActiveTab] = useState<FilterTab>("recent");
  const [mentionIndex, setMentionIndex] = useState(0);
  const [slashIndex, setSlashIndex] = useState(0);
  const [fileIndex, setFileIndex] = useState(0);
  const sendMessage = useSessionContext((state) => state.sendMessage);
  const {
    input,
    isSubmitting,
    parts,
    error,
    setInput,
    setError,
    submit,
    dismiss,
    reset,
  } = useCommandBar(sendMessage);
  const serverInfo = useServerContext((state) => state.info);
  const extensions = useExtensionContext((state) => state.extensions);
  const extensionsLoaded = useExtensionContext((state) => state.isLoaded);
  const fetchExtensions = useExtensionContext((state) => state.fetchExtensions);
  const openExtension = useExtensionContext((state) => state.openExtension);

  // Filesystem search
  const searchResults = useFileSystemContext((state) => state.searchResults);
  const isSearching = useFileSystemContext((state) => state.isSearching);
  const searchError = useFileSystemContext((state) => state.searchError);
  const searchFiles = useFileSystemContext((state) => state.search);
  const clearSearch = useFileSystemContext((state) => state.clearSearch);

  const mentionState = useMemo(() => getMentionState(input), [input]);
  const slashState = useMemo(() => getSlashState(input), [input]);
  const hashState = useMemo(() => getHashState(input), [input]);
  const slashCommands = useMemo(
    () => [
      { id: "settings", name: "Settings", description: "Open the settings window" },
    ],
    []
  );

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
      ? normalizeQuery(input)
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
  }, [activeTab, extensions, input, mentionState]);

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
      ? normalizeQuery(input)
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
  }, [activeTab, input, slashCommands, slashState]);

  const showExtensionsList = activeTab === "extensions" || !!mentionState;
  const showCommandsList = !showExtensionsList && filteredSlashCommands.length > 0;
  const showFilesList = activeTab === "files" || !!hashState;

  // Trigger file search when hash state changes
  const searchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (!hashState && activeTab !== "files") {
      clearSearch();
      return;
    }
    const query = hashState?.query ?? input;
    if (!query.trim()) {
      clearSearch();
      return;
    }
    // Debounce search
    if (searchDebounceRef.current) {
      clearTimeout(searchDebounceRef.current);
    }
    searchDebounceRef.current = setTimeout(() => {
      searchFiles({ query: query.trim(), maxResults: 50 });
    }, 150);
    return () => {
      if (searchDebounceRef.current) {
        clearTimeout(searchDebounceRef.current);
      }
    };
  }, [hashState?.query, activeTab, input, searchFiles, clearSearch]);

  // Reset file index when results change
  useEffect(() => {
    setFileIndex(0);
  }, [searchResults]);

  const fileEntries = searchResults?.results ?? [];

  const insertSigilAtCursor = (sigil: "@" | "/" | "#") => {
    const node = inputRef.current;
    const start = node?.selectionStart ?? input.length;
    const end = node?.selectionEnd ?? input.length;
    let replaceStart = start;
    let replaceEnd = end;

    if (start === end) {
      const prevChar = start > 0 ? input[start - 1] : "";
      const nextChar = start < input.length ? input[start] : "";
      if (prevChar === "@" || prevChar === "/" || prevChar === "#") {
        replaceStart = start - 1;
        replaceEnd = start;
      } else if (nextChar === "@" || nextChar === "/" || nextChar === "#") {
        replaceStart = start;
        replaceEnd = start + 1;
      }
    }

    const nextValue = `${input.slice(0, replaceStart)}${sigil}${input.slice(replaceEnd)}`;
    const caret = replaceStart + sigil.length;
    setInput(nextValue);
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
      input.length === 0 &&
      (activeTab === "extensions" || activeTab === "commands" || activeTab === "files")
    ) {
      e.preventDefault();
      setActiveTab("recent");
      return;
    }

    if (showFilesList && fileEntries.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setFileIndex((idx) => (idx + 1) % fileEntries.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setFileIndex((idx) =>
          idx <= 0 ? fileEntries.length - 1 : idx - 1
        );
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const selected = fileEntries[fileIndex];
        if (selected) {
          // Insert the file path into the input
          const pathToInsert = selected.path;
          const beforeHash = hashState ? input.slice(0, hashState.start) : input;
          const nextValue = `${beforeHash}${pathToInsert} `;
          setInput(nextValue);
          setActiveTab("recent");
          clearSearch();
        }
        return;
      }
    }

    if (showExtensionsList && filteredExtensions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setMentionIndex((idx) => (idx + 1) % filteredExtensions.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setMentionIndex((idx) =>
          idx <= 0 ? filteredExtensions.length - 1 : idx - 1
        );
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const selected = filteredExtensions[mentionIndex];
        if (selected) {
          const nextValue = mentionState
            ? applyMention(input, mentionState, selected.name)
            : appendMention(input, selected.name);
          setInput(nextValue);
          setActiveTab("recent");
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
          const nextValue = slashState
            ? applySlashCommand(input, slashState, selected.id)
            : appendSlashCommand(input, selected.id);
          setInput(nextValue);
          setActiveTab("recent");
          return;
        }
      }
    }

    switch (e.key) {
      case "Enter":
        e.preventDefault();
        {
          const trimmed = input.trim();
          const mentionExtensionId = findExactMentionExtensionId(trimmed, extensions);
          if (mentionExtensionId) {
            openExtension(mentionExtensionId)
              .then(() => {
                reset();
              })
              .catch((err) => {
                setError(String(err));
              });
            return;
          }
          const resolved = resolveMentions(input, extensions);
          submit(resolved);
        }
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  };

  const showResponses = parts.length > 0 || !!error;

  return (
    <main className="app-shell">
      <CommandPaletteShell className={`app-shell-panel ${styles.shell}`}>
      <HeaderArea>
        <div className={styles.headerRow}>
          <SearchField
            className={styles.searchField}
            icon={<Icon>{SearchIcon}</Icon>}
            placeholder="How can I help..."
            inputRef={inputRef}
            inputProps={{
              id: inputId,
              value: input,
              onChange: (e) => setInput(e.target.value),
              onKeyDown: handleKeyDown,
              disabled: isSubmitting,
              spellCheck: false,
              autoComplete: "off",
            }}
          />
          <StatusBadge
            status={serverInfo ? "good" : "warn"}
            label={serverInfo ? "Server online" : "Server offline"}
          />
        </div>
        <Divider />
      </HeaderArea>

      <FilterArea>
        <div className={styles.filterRow}>
          <ChipGroup>
            <Chip
              label="Recent"
              active={activeTab === "recent" && !mentionState && !slashState}
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
              label="Files"
              active={activeTab === "files" || !!hashState}
              onClick={() => {
                setActiveTab("files");
                insertSigilAtCursor("#");
              }}
            />
          </ChipGroup>
          {isSubmitting ? <Badge>Working...</Badge> : null}
        </div>
      </FilterArea>

      <ContentArea className={styles.content}>
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
                      const nextValue = mentionState
                        ? applyMention(input, mentionState, extension.name)
                        : appendMention(input, extension.name);
                      setInput(nextValue);
                      setActiveTab("recent");
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
                    const nextValue = slashState
                      ? applySlashCommand(input, slashState, command.id)
                      : appendSlashCommand(input, command.id);
                    setInput(nextValue);
                    setActiveTab("recent");
                  }}
                />
              ))}
            </ListSection>
          ) : null}

          {showFilesList ? (
            <ListSection label={isSearching ? "Searching..." : `Files${searchResults ? ` (${searchResults.count})` : ""}`}>
              {searchError ? (
                <Text size="sm" tone="secondary">
                  {searchError}
                </Text>
              ) : fileEntries.length > 0 ? (
                fileEntries.map((entry, index) => (
                  <ListItem
                    key={entry.path}
                    title={entry.name}
                    subtitle={entry.path}
                    icon={
                      <IconContainer>
                        <Icon>{entry.type === "directory" ? FolderIcon : FileIcon}</Icon>
                      </IconContainer>
                    }
                    selected={index === fileIndex}
                    onMouseDown={(event) => {
                      event.preventDefault();
                      // TODO: Handle file selection (open, insert path, etc.)
                    }}
                  />
                ))
              ) : hashState?.query ? (
                <Text size="sm" tone="secondary">
                  {isSearching ? "Searching..." : "No files found."}
                </Text>
              ) : (
                <Text size="sm" tone="secondary">
                  Type to search files...
                </Text>
              )}
            </ListSection>
          ) : null}

          {(showExtensionsList || showCommandsList || showFilesList) && showResponses ? <Divider /> : null}

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
          ) : !showExtensionsList && !showCommandsList ? (
            <Text size="sm" tone="secondary">
              Type a command, use @ to target an extension, or / for shortcuts.
            </Text>
          ) : null}
        </div>
      </ContentArea>

      <FooterArea>
        <HintBar
          left={
            <>
              <HintItem label="Navigate" keyHint={<KeyHint keys={["↑", "↓"]} />} />
              <HintItem label="Enter" keyHint={<KeyHint keys="↵" />} />
              <HintItem label="Extensions" keyHint={<KeyHint keys="@" />} />
              <HintItem label="Command" keyHint={<KeyHint keys="/" />} />
              <HintItem label="Files" keyHint={<KeyHint keys="#" />} />
            </>
          }
          right={<CloseButton keyLabel="esc" onClick={dismiss} />}
        />
      </FooterArea>
      </CommandPaletteShell>
    </main>
  );
}
