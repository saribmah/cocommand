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
  ActionRow,
  Badge,
  ButtonPrimary,
  ButtonSecondary,
  Chip,
  ChipGroup,
  CloseButton,
  CommandPaletteShell,
  ContentArea,
  Divider,
  ErrorCard,
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
  ReasoningCard,
  ResponseBlock,
  ResponseHeader,
  ResponseStack,
  SearchField,
  StatusBadge,
  Text,
  ToolCallCard,
} from "@cocommand/ui";
import { MarkdownResponseCard } from "../../components/MarkdownResponseCard";
import { useExtensionContext } from "../extension/extension.context";
import { useSessionContext } from "../session/session.context";
import { useServerContext } from "../server/server.context";
import { useCommandBar } from "./commandbar";
import type { MessagePart, SourcePart } from "../session/session.types";
import styles from "./command.module.css";

const SearchIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
    <circle cx="11" cy="11" r="6" />
    <path d="M20 20l-3.8-3.8" />
  </svg>
);

const ExtensionIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
    <rect x="4" y="4" width="7" height="7" rx="2" />
    <rect x="13" y="4" width="7" height="7" rx="2" />
    <rect x="4" y="13" width="7" height="7" rx="2" />
    <rect x="13" y="13" width="7" height="7" rx="2" />
  </svg>
);

const CommandIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
    <path d="M5 7h14" />
    <path d="M5 12h14" />
    <path d="M5 17h10" />
  </svg>
);

const ArrowIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
    <path d="M7 17l10-10" />
    <path d="M9 7h8v8" />
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

type ToolState = "pending" | "running" | "success" | "error";

type DisplayItem =
  | { kind: "text"; text: string }
  | { kind: "reasoning"; text: string }
  | {
      kind: "tool";
      toolName: string;
      toolId: string;
      state: ToolState;
      params?: string;
      result?: string;
      errorMessage?: string;
    }
  | { kind: "file"; name: string; mediaType?: string | null }
  | { kind: "source"; label: string; body: string };

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
  return lines.length > 0 ? lines.join("\n") : part.source_type;
}

function toDisplayItems(parts: MessagePart[]): DisplayItem[] {
  const items: DisplayItem[] = [];
  const toolIndex = new Map<string, number>();

  const appendText = (text: string) => {
    if (!text) return;
    const last = items[items.length - 1];
    if (last && last.kind === "text") {
      last.text += text;
    } else {
      items.push({ kind: "text", text });
    }
  };

  const appendReasoning = (text: string) => {
    if (!text) return;
    const last = items[items.length - 1];
    if (last && last.kind === "reasoning") {
      last.text += text;
    } else {
      items.push({ kind: "reasoning", text });
    }
  };

  for (const part of parts) {
    switch (part.type) {
      case "text":
        appendText(part.text);
        break;
      case "reasoning":
        appendReasoning(part.text);
        break;
      case "tool-call": {
        const toolId = part.call_id || `tool_${items.length}`;
        const entry: DisplayItem = {
          kind: "tool",
          toolName: part.tool_name,
          toolId,
          state: "running",
          params: formatPayload(part.input),
        };
        toolIndex.set(toolId, items.length);
        items.push(entry);
        break;
      }
      case "tool-result": {
        const toolId = part.call_id || `tool_${items.length}`;
        const index = toolIndex.get(toolId);
        const state: ToolState = part.is_error ? "error" : "success";
        const payload = formatPayload(part.output);
        if (index !== undefined) {
          const existing = items[index];
          if (existing.kind === "tool") {
            existing.state = state;
            if (state === "error") {
              existing.errorMessage = payload;
            } else {
              existing.result = payload;
            }
          }
        } else {
          items.push({
            kind: "tool",
            toolName: part.tool_name,
            toolId,
            state,
            result: state === "success" ? payload : undefined,
            errorMessage: state === "error" ? payload : undefined,
          });
        }
        break;
      }
      case "file": {
        items.push({
          kind: "file",
          name: part.name ?? "Untitled file",
          mediaType: part.media_type,
        });
        break;
      }
      case "source": {
        items.push({
          kind: "source",
          label: "Source",
          body: formatSourceBody(part),
        });
        break;
      }
      default:
        break;
    }
  }

  return items;
}

function formatFileType(mediaType?: string | null): string | undefined {
  if (!mediaType) return undefined;
  const bits = mediaType.split("/");
  if (bits.length < 2) return mediaType.toUpperCase();
  return bits[1].toUpperCase();
}

export function CommandView() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputId = useId();
  const [mentionIndex, setMentionIndex] = useState(0);
  const [slashIndex, setSlashIndex] = useState(0);
  const sendMessage = useSessionContext((state) => state.sendMessage);
  const {
    input,
    isSubmitting,
    parts,
    pendingConfirmation,
    followUpActive,
    error,
    setInput,
    setError,
    submit,
    dismiss,
    confirmPending,
    cancelPending,
    reset,
  } = useCommandBar(sendMessage);
  const serverInfo = useServerContext((state) => state.info);
  const extensions = useExtensionContext((state) => state.extensions);
  const extensionsLoaded = useExtensionContext((state) => state.isLoaded);
  const fetchExtensions = useExtensionContext((state) => state.fetchExtensions);
  const openExtension = useExtensionContext((state) => state.openExtension);

  const mentionState = useMemo(() => getMentionState(input), [input]);
  const slashState = useMemo(() => getSlashState(input), [input]);
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
  }, [parts, pendingConfirmation]);

  const filteredExtensions = useMemo(() => {
    if (!mentionState) return [];
    const query = normalizeQuery(mentionState.query);
    const ranked = extensions
      .map((extension) => ({
        extension,
        score: matchScore(query, extension.name, extension.id, extension.kind),
      }))
      .filter((entry) => (query.length === 0 ? true : entry.score >= 0))
      .sort((a, b) => b.score - a.score);
    return ranked.slice(0, 8).map((entry) => entry.extension);
  }, [extensions, mentionState]);

  useEffect(() => {
    if (mentionState) {
      setMentionIndex(0);
    }
  }, [mentionState?.query, mentionState?.start]);

  useEffect(() => {
    if (slashState) {
      setSlashIndex(0);
    }
  }, [slashState?.query, slashState?.start]);

  const filteredSlashCommands = useMemo(() => {
    if (!slashState) return [];
    const query = normalizeQuery(slashState.query);
    const ranked = slashCommands
      .map((command) => ({
        command,
        score: matchScore(query, command.name, command.id, command.description),
      }))
      .filter((entry) => (query.length === 0 ? true : entry.score >= 0))
      .sort((a, b) => b.score - a.score);
    return ranked.slice(0, 6).map((entry) => entry.command);
  }, [slashCommands, slashState]);

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (mentionState && filteredExtensions.length > 0) {
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
          const nextValue = applyMention(input, mentionState, selected.name);
          setInput(nextValue);
        }
        return;
      }
    }

    if (!mentionState && slashState && filteredSlashCommands.length > 0) {
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
        const trimmed = input.trim();
        if (selected && trimmed !== `/${selected.id}`) {
          e.preventDefault();
          const nextValue = applySlashCommand(input, slashState, selected.id);
          setInput(nextValue);
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

  const displayItems = useMemo(() => toDisplayItems(parts), [parts]);
  const showMentionList = mentionState && filteredExtensions.length > 0;
  const showSlashList = !mentionState && slashState && filteredSlashCommands.length > 0;
  const showResponses = displayItems.length > 0 || pendingConfirmation || !!error;

  return (
    <main className={styles.host}>
      <CommandPaletteShell className={styles.shell}>
      <HeaderArea>
        <div className={styles.headerRow}>
          <SearchField
            className={styles.searchField}
            icon={<Icon>{SearchIcon}</Icon>}
            placeholder={followUpActive ? "Refine the previous result..." : "How can I help..."}
            inputProps={{
              id: inputId,
              value: input,
              onChange: (e) => setInput(e.target.value),
              onKeyDown: handleKeyDown,
              disabled: isSubmitting || !!pendingConfirmation,
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
            <Chip label="Recent" active={!mentionState && !slashState} />
            <Chip label="Extensions" active={!!mentionState} />
            <Chip label="Commands" active={!!slashState && !mentionState} />
          </ChipGroup>
          {followUpActive ? <Badge tone="warn">Follow-up</Badge> : null}
          {isSubmitting ? <Badge>Working...</Badge> : null}
        </div>
      </FilterArea>

      <ContentArea className={styles.content}>
        <div className={styles.scrollArea} ref={scrollRef}>
          {showMentionList ? (
            <ListSection label="Extensions">
              {filteredExtensions.map((extension, index) => (
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
                    const nextValue = applyMention(input, mentionState, extension.name);
                    setInput(nextValue);
                  }}
                />
              ))}
            </ListSection>
          ) : null}

          {showSlashList ? (
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
                    const nextValue = applySlashCommand(input, slashState, command.id);
                    setInput(nextValue);
                  }}
                />
              ))}
            </ListSection>
          ) : null}

          {(showMentionList || showSlashList) && showResponses ? <Divider /> : null}

          {showResponses ? (
            <ResponseStack>
              {error ? <ErrorCard message={error} /> : null}

              {pendingConfirmation ? (
                <ResponseBlock className={styles.responseBlock}>
                  <ResponseHeader label={pendingConfirmation.title} />
                  <Text size="sm" tone="secondary">
                    {pendingConfirmation.body}
                  </Text>
                  <ActionRow>
                    <ButtonSecondary onClick={() => cancelPending()}>
                      Cancel
                    </ButtonSecondary>
                    <ButtonPrimary onClick={() => confirmPending()}>
                      Confirm
                    </ButtonPrimary>
                  </ActionRow>
                </ResponseBlock>
              ) : null}

              {displayItems.map((item, index) => {
                switch (item.kind) {
                  case "text":
                    return (
                      <MarkdownResponseCard key={`text-${index}`} body={item.text} />
                    );
                  case "reasoning":
                    return (
                      <ReasoningCard
                        key={`reasoning-${index}`}
                        reasoning={item.text}
                      />
                    );
                  case "tool":
                    return (
                      <ToolCallCard
                        key={`tool-${item.toolId}-${index}`}
                        toolName={item.toolName}
                        toolId={item.toolId}
                        state={item.state}
                        params={item.params}
                        result={item.result}
                        errorMessage={item.errorMessage}
                      />
                    );
                  case "file":
                    return (
                      <FileCard
                        key={`file-${index}`}
                        fileName={item.name}
                        fileType={formatFileType(item.mediaType)}
                      />
                    );
                  case "source":
                    return (
                      <MarkdownResponseCard
                        key={`source-${index}`}
                        label={item.label}
                        body={item.body}
                      />
                    );
                  default:
                    return null;
                }
              })}
            </ResponseStack>
          ) : !showMentionList && !showSlashList ? (
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
              <HintItem label="Navigate" keyHint={<KeyHint keys={["Up", "Down"]} />} />
              <HintItem label="Enter" keyHint={<KeyHint keys="Enter" />} />
              <HintItem label="Extensions" keyHint={<KeyHint keys="@" />} />
              <HintItem label="Command" keyHint={<KeyHint keys="/" />} />
            </>
          }
          right={<CloseButton onClick={dismiss} />}
        />
      </FooterArea>
      </CommandPaletteShell>
    </main>
  );
}
