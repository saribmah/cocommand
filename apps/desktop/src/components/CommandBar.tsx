import { useRef, useEffect, useMemo, useState, type KeyboardEvent } from "react";
import { useCommandBar } from "../state/commandbar";
import { useServerStore } from "../state/server";
import { useApplicationStore } from "../state/applications";
import { ResultCard } from "./ResultCard";
import { ConfirmPanel } from "./ConfirmPanel";
import { ApplicationPicker } from "./ApplicationPicker";
import { SlashCommandPicker } from "./SlashCommandPicker";
import "../styles/commandbar.css";

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
  applications: { id: string; name: string }[]
): string {
  return text.replace(/@([^\s@]+)/g, (full, name) => {
    const normalized = String(name).trim().toLowerCase();
    const match = applications.find(
      (app) =>
        app.name.toLowerCase() === normalized || app.id.toLowerCase() === normalized
    );
    if (!match) return full;
    return `@${match.id}`;
  });
}

function findExactMentionId(
  text: string,
  applications: { id: string; name: string }[]
): string | null {
  const trimmed = text.trim();
  if (!trimmed.startsWith("@")) return null;
  const mention = trimmed.slice(1).trim();
  const normalized = mention.toLowerCase();
  const match = applications.find(
    (app) =>
      app.id.toLowerCase() === normalized || app.name.toLowerCase() === normalized
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

export function CommandBar() {
  const inputRef = useRef<HTMLInputElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);
  const [mentionIndex, setMentionIndex] = useState(0);
  const [slashIndex, setSlashIndex] = useState(0);
  const {
    input,
    isSubmitting,
    results,
    pendingConfirmation,
    followUpActive,
    setInput,
    setResults,
    submit,
    dismiss,
    dismissResult,
    confirmPending,
    cancelPending,
    reset,
  } = useCommandBar();
  const serverInfo = useServerStore((state) => state.info);
  const applications = useApplicationStore((state) => state.applications);
  const applicationsLoaded = useApplicationStore((state) => state.isLoaded);
  const fetchApplications = useApplicationStore((state) => state.fetchApplications);
  const openApplication = useApplicationStore((state) => state.openApplication);

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
    fetchApplications();
  }, [serverInfo, fetchApplications]);

  useEffect(() => {
    if (!mentionState) return;
    if (applicationsLoaded) return;
    fetchApplications();
  }, [mentionState, applicationsLoaded, fetchApplications]);

  useEffect(() => {
    inputRef.current?.focus();
  }, [results]);

  useEffect(() => {
    const node = resultsRef.current;
    if (!node) return;
    requestAnimationFrame(() => {
      node.scrollTop = node.scrollHeight;
    });
  }, [results]);

  useEffect(() => {
    if (mentionState) {
      console.log("[mentions] state", mentionState);
    }
  }, [mentionState]);
  const filteredApplications = useMemo(() => {
    if (!mentionState) return [];
    const query = normalizeQuery(mentionState.query);
    const ranked = applications
      .map((app) => ({
        app,
        score: matchScore(query, app.name, app.id, app.kind),
      }))
      .filter((entry) => (query.length === 0 ? true : entry.score >= 0))
      .sort((a, b) => b.score - a.score);
    return ranked.slice(0, 8).map((entry) => entry.app);
  }, [applications, mentionState]);

  useEffect(() => {
    if (!mentionState) return;
    console.log("[mentions] apps", applications.length, "filtered", filteredApplications.length);
  }, [mentionState, applications.length, filteredApplications.length]);

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
    if (mentionState && filteredApplications.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setMentionIndex((idx) => (idx + 1) % filteredApplications.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setMentionIndex((idx) =>
          idx <= 0 ? filteredApplications.length - 1 : idx - 1
        );
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const selected = filteredApplications[mentionIndex];
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
          const mentionId = findExactMentionId(trimmed, applications);
          if (mentionId) {
            const appId = mentionId;
            openApplication(appId)
              .then(() => {
                reset();
              })
              .catch((err) => {
                setResults([
                  {
                    type: "error",
                    title: "Error",
                    body: String(err),
                  },
                ]);
              });
            return;
          }
          const resolved = resolveMentions(input, applications);
          submit(resolved);
        }
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  };
  return (
    <div className="command-bar">
      <div className="command-input-wrapper">
        {followUpActive && (
          <span className="follow-up-badge">Follow-up</span>
        )}
        <input
          ref={inputRef}
          className="command-input"
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={followUpActive ? "Refine the previous result\u2026" : "How can I help..."}
          disabled={isSubmitting || !!pendingConfirmation}
          spellCheck={false}
          autoComplete="off"
        />
        <span
          className="server-status-badge"
          data-status={serverInfo ? "online" : "offline"}
        >
          {serverInfo ? "Server online" : "Server offline"}
        </span>
      </div>
      {mentionState && (
        <ApplicationPicker
          applications={filteredApplications}
          selectedIndex={mentionIndex}
          onSelect={(app) => {
            const nextValue = applyMention(input, mentionState, app.name);
            setInput(nextValue);
          }}
        />
      )}
      {!mentionState && slashState && (
        <SlashCommandPicker
          commands={filteredSlashCommands}
          selectedIndex={slashIndex}
          onSelect={(command) => {
            const nextValue = applySlashCommand(input, slashState, command.id);
            setInput(nextValue);
          }}
        />
      )}
      <div className="command-results" ref={resultsRef}>
        {pendingConfirmation && (
          <ConfirmPanel
            confirmation={pendingConfirmation}
            onConfirm={confirmPending}
            onCancel={cancelPending}
          />
        )}
        <ResultCard results={results} onDismiss={dismissResult} />
      </div>
    </div>
  );
}
