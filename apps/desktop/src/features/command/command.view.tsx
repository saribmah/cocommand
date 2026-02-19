import "@cocommand/ui";
import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { hideWindow, openExtensionWindow } from "../../lib/ipc";
import {
  CloseButton,
  CommandPaletteShell,
  FooterArea,
  HintBar,
  HintItem,
  KeyHint,
} from "@cocommand/ui";
import { useApplicationContext } from "../application/application.context";
import type { ApplicationInfo } from "../application/application.types";
import { useExtensionContext } from "../extension/extension.context";
import { useSessionContext } from "../session/session.context";
import { useServerContext } from "../server/server.context";
import { useCommandContext } from "./command.context";
import type {
  ExtensionPartInput,
  MessagePartInput,
} from "./command.types";
import { hasExtensionView } from "../extension/extension-views";
import type { ComposerActions } from "./composer-actions";
import {
  buildTagSegments,
  commitComposerParts,
  findExactMentionExtensionId,
  getActiveText,
  getActiveTextPartIndex,
  getHashState,
  getMentionState,
  getSlashState,
  getStarState,
  insertPartAfterActiveText,
  matchScore,
  normalizeQuery,
  removeTaggedPartBySource,
  removeTrailingSigilQuery,
  updateActiveText,
  type ComposerTagSegment,
  type FilterTab,
} from "./composer-utils";
import { Composer } from "./components/Composer";
import { PillArea } from "./components/PillArea";
import { RenderingArea } from "./components/RenderingArea";
import styles from "./command.module.css";

function cloneMessagePartInput(part: MessagePartInput): MessagePartInput {
  switch (part.type) {
    case "text":
      return { ...part };
    case "extension":
      return {
        ...part,
        source: part.source ? { ...part.source } : part.source,
      };
    case "file":
      return {
        ...part,
        source: part.source ? { ...part.source } : part.source,
      };
    default:
      return part;
  }
}

function cloneMessagePartInputs(parts: MessagePartInput[]): MessagePartInput[] {
  return parts.map(cloneMessagePartInput);
}

export function CommandView() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const inputId = useId();
  const [activeTab, setActiveTab] = useState<FilterTab>("recent");
  const [mentionIndex, setMentionIndex] = useState(0);
  const [slashIndex, setSlashIndex] = useState(0);
  const [applicationIndex, setApplicationIndex] = useState(0);
  const [inputHistoryCursor, setInputHistoryCursor] = useState<number | null>(null);
  const draftBeforeHistoryRef = useRef<MessagePartInput[] | null>(null);
  const isApplyingHistoryRef = useRef(false);

  const draftParts = useCommandContext((state) => state.draftParts);
  const setDraftParts = useCommandContext((state) => state.setDraftParts);
  const isSubmitting = useCommandContext((state) => state.isSubmitting);
  const parts = useCommandContext((state) => state.parts);
  const turns = useCommandContext((state) => state.turns);
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
  // Subscribe to viewLoadVersion so hasExtensionView() re-evaluates after dynamic loads
  useExtensionContext((state) => state.viewLoadVersion);

  const applications = useApplicationContext((state) => state.applications);
  const applicationsCount = useApplicationContext((state) => state.count);
  const applicationsLoaded = useApplicationContext((state) => state.isLoaded);
  const applicationsLoading = useApplicationContext((state) => state.isLoading);
  const applicationsError = useApplicationContext((state) => state.error);
  const fetchApplications = useApplicationContext((state) => state.fetchApplications);
  const openApplication = useApplicationContext((state) => state.openApplication);
  const clearApplications = useApplicationContext((state) => state.clear);

  // ---------------------------------------------------------------------------
  // Derived state
  // ---------------------------------------------------------------------------

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

  const activeView: FilterTab = useMemo(() => {
    if (mentionState) return "extensions";
    if (slashState) return "commands";
    if (starState) return "applications";
    if (hashState) return "ext:filesystem";
    return activeTab;
  }, [mentionState, slashState, starState, hashState, activeTab]);

  const slashCommands = useMemo(
    () => [{ id: "settings", name: "Settings", description: "Open the settings window" }],
    []
  );
  const submittedInputHistory = useMemo(
    () => turns.map((turn) => turn.inputParts),
    [turns]
  );

  // ---------------------------------------------------------------------------
  // Composer helpers
  // ---------------------------------------------------------------------------

  const clearInputHistoryNavigation = () => {
    setInputHistoryCursor(null);
    draftBeforeHistoryRef.current = null;
  };

  const applyComposerParts = (next: MessagePartInput[]) => {
    if (!isApplyingHistoryRef.current) {
      clearInputHistoryNavigation();
    }
    setDraftParts(commitComposerParts(next));
  };

  const updateComposerText = (value: string) => {
    applyComposerParts(updateActiveText(composerParts, value));
  };

  const applyHistoryDraft = (nextDraft: MessagePartInput[]) => {
    isApplyingHistoryRef.current = true;
    setDraftParts(cloneMessagePartInputs(nextDraft));
    requestAnimationFrame(() => {
      const node = inputRef.current;
      if (node) {
        node.focus();
        const caret = node.value.length;
        node.setSelectionRange(caret, caret);
      }
      isApplyingHistoryRef.current = false;
    });
  };

  const focusInput = () => {
    requestAnimationFrame(() => {
      inputRef.current?.focus();
    });
  };

  // ---------------------------------------------------------------------------
  // Effects
  // ---------------------------------------------------------------------------

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
  }, [inputId, parts, turns]);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node) return;
    requestAnimationFrame(() => {
      node.scrollTop = node.scrollHeight;
    });
  }, [parts, turns, error]);

  useEffect(() => {
    if (activeView !== "recent") {
      clearInputHistoryNavigation();
    }
  }, [activeView]);

  // ---------------------------------------------------------------------------
  // Filtered lists
  // ---------------------------------------------------------------------------

  const readyExtensions = useMemo(
    () => extensions.filter((ext) => ext.status === "ready"),
    [extensions],
  );

  const filteredExtensions = useMemo(() => {
    const query = mentionState
      ? normalizeQuery(mentionState.query)
      : activeTab === "extensions"
      ? normalizeQuery(activeText)
      : "";
    if (!mentionState && activeTab !== "extensions") return [];
    if (!mentionState) {
      if (!query) {
        return [...readyExtensions].sort((a, b) => a.name.localeCompare(b.name));
      }
      const ranked = readyExtensions
        .map((extension) => ({
          extension,
          score: matchScore(query, extension.name, extension.id, extension.kind),
        }))
        .filter((entry) => entry.score >= 0)
        .sort((a, b) => b.score - a.score);
      return ranked.slice(0, 8).map((entry) => entry.extension);
    }
    const ranked = readyExtensions
      .map((extension) => ({
        extension,
        score: matchScore(query, extension.name, extension.id, extension.kind),
      }))
      .filter((entry) => (query.length === 0 ? true : entry.score >= 0))
      .sort((a, b) => b.score - a.score);
    return ranked.slice(0, 8).map((entry) => entry.extension);
  }, [activeTab, activeText, readyExtensions, mentionState]);

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

  // ---------------------------------------------------------------------------
  // View routing flags
  // ---------------------------------------------------------------------------

  const showExtensionView = activeView.startsWith("ext:");
  const activeExtensionId = showExtensionView ? activeView.slice(4) : null;
  const showExtensionsList = activeView === "extensions";
  const showCommandsList = activeView === "commands";
  const showApplicationsList = activeView === "applications";

  useEffect(() => {
    if (!showApplicationsList) return;
    setApplicationIndex(0);
  }, [showApplicationsList, starState?.query, starState?.start, activeView]);

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

  // ---------------------------------------------------------------------------
  // Callbacks
  // ---------------------------------------------------------------------------

  const executeSlashCommand = (id: string) => {
    if (id !== "settings") return;
    openExtensionWindow({
      extensionId: "workspace",
      title: "Settings",
      width: 720,
      height: 520,
    })
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

  const removeTaggedSegment = (segment: ComposerTagSegment) => {
    if (segment.type !== "extension" && segment.type !== "file") return;
    const next = removeTaggedPartBySource(composerParts, {
      type: segment.type,
      start: segment.start,
      end: segment.end,
    });
    applyComposerParts(next);

    // If removing the extension whose view is currently active, close it
    if (segment.type === "extension" && activeTab === `ext:${segment.part.extensionId}`) {
      setActiveTab("recent");
    }

    focusInput();
  };

  const handleClickSegment = (segment: ComposerTagSegment) => {
    if (segment.type === "extension" && hasExtensionView(segment.part.extensionId)) {
      setActiveTab(`ext:${segment.part.extensionId}`);
    }
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

  // ---------------------------------------------------------------------------
  // ComposerActions (for extensions)
  // ---------------------------------------------------------------------------

  const stateRef = useRef({ activeText, composerParts, hashState, mentionState });
  stateRef.current = { activeText, composerParts, hashState, mentionState };

  const composerActions: ComposerActions = useMemo(() => ({
    addPart: (part) => {
      const { activeText: text, composerParts: parts, hashState: hash } = stateRef.current;
      const normalizedName =
        part.type === "file"
          ? (part.name.trim().length > 0
              ? part.name
              : part.path.split("/").filter(Boolean).pop() ?? part.path)
          : part.name;

      const nextText = removeTrailingSigilQuery(text, hash);
      let nextParts = updateActiveText(parts, nextText);
      const partWithSource = {
        ...part,
        name: normalizedName,
        source: {
          value: part.type === "file" ? `#${normalizedName}` : `@${part.extensionId}`,
          start: 0,
          end: 0,
        },
      };
      nextParts = insertPartAfterActiveText(nextParts, partWithSource);
      applyComposerParts(nextParts);
      setActiveTab("recent");
      requestAnimationFrame(() => {
        inputRef.current?.focus();
      });
    },
    removePart: (match) => {
      const { composerParts: parts } = stateRef.current;
      const index = parts.findIndex((p) => {
        if (p.type !== match.type) return false;
        if (p.type === "file" && match.type === "file") return p.name === match.name;
        if (p.type === "extension" && match.type === "extension") return p.name === match.name;
        return false;
      });
      if (index < 0) return;
      const next = [...parts];
      next.splice(index, 1);
      applyComposerParts(next);
    },
    setActiveTab: (tab) => setActiveTab(tab as FilterTab),
    focusInput,
  }), []);

  // ---------------------------------------------------------------------------
  // Keyboard handler
  // ---------------------------------------------------------------------------

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

      // When closing an extension view, also remove its tag from the composer
      if (activeTab.startsWith("ext:")) {
        const extId = activeTab.slice(4);
        const activeIndex = getActiveTextPartIndex(composerParts);
        // Find the extension part matching this view (search backwards from active text)
        let removeIdx = -1;
        for (let i = activeIndex - 1; i >= 0; i--) {
          const p = composerParts[i];
          if (p?.type === "extension" && p.extensionId === extId) {
            removeIdx = i;
            break;
          }
        }
        if (removeIdx >= 0) {
          const next = [...composerParts];
          next.splice(removeIdx, 1);
          applyComposerParts(next);
        }
      }

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

    const hasModifierKey = e.metaKey || e.ctrlKey || e.altKey || e.shiftKey;
    const canNavigateInputHistory =
      activeView === "recent" &&
      !showExtensionView &&
      !showExtensionsList &&
      !showCommandsList &&
      !showApplicationsList &&
      !hasModifierKey;

    if (canNavigateInputHistory && e.key === "ArrowUp") {
      if (submittedInputHistory.length === 0) return;
      e.preventDefault();
      if (inputHistoryCursor === null) {
        draftBeforeHistoryRef.current = cloneMessagePartInputs(draftParts);
        const nextCursor = submittedInputHistory.length - 1;
        setInputHistoryCursor(nextCursor);
        applyHistoryDraft(submittedInputHistory[nextCursor] ?? []);
        return;
      }
      const nextCursor = Math.max(inputHistoryCursor - 1, 0);
      setInputHistoryCursor(nextCursor);
      applyHistoryDraft(submittedInputHistory[nextCursor] ?? []);
      return;
    }

    if (canNavigateInputHistory && e.key === "ArrowDown") {
      if (inputHistoryCursor === null || submittedInputHistory.length === 0) return;
      e.preventDefault();
      if (inputHistoryCursor >= submittedInputHistory.length - 1) {
        setInputHistoryCursor(null);
        const draftBeforeHistory = draftBeforeHistoryRef.current;
        draftBeforeHistoryRef.current = null;
        applyHistoryDraft(draftBeforeHistory ?? [{ type: "text", text: "" }]);
        return;
      }
      const nextCursor = inputHistoryCursor + 1;
      setInputHistoryCursor(nextCursor);
      applyHistoryDraft(submittedInputHistory[nextCursor] ?? []);
      return;
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
          clearInputHistoryNavigation();
          submit(sendMessage);
        }
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  const placeholder =
    activeText.length === 0 && tagSegments.length === 0
      ? "How can I help..."
      : "";

  return (
    <main className="app-shell">
      <CommandPaletteShell className={`app-shell-panel ${styles.shell}`}>
        <Composer
          inputId={inputId}
          inputRef={inputRef}
          activeText={activeText}
          tagSegments={tagSegments}
          isSubmitting={isSubmitting}
          serverOnline={!!serverInfo}
          placeholder={placeholder}
          onTextChange={updateComposerText}
          onKeyDown={handleKeyDown}
          onRemoveSegment={removeTaggedSegment}
          onClickSegment={handleClickSegment}
        />

        <PillArea
          activeView={activeView}
          isSubmitting={isSubmitting}
          extensionPills={extensionPills}
          onTabChange={setActiveTab}
          onExtensionsClick={() => {
            setActiveTab("extensions");
            fetchExtensions();
            insertSigilAtCursor("@");
          }}
          onCommandsClick={() => {
            setActiveTab("commands");
            insertSigilAtCursor("/");
          }}
          onApplicationsClick={() => {
            setActiveTab("applications");
            fetchApplications().catch(() => {
              // application store already tracks error state
            });
            insertSigilAtCursor("*");
          }}
        />

        <RenderingArea
          showExtensionView={showExtensionView}
          activeExtensionId={activeExtensionId}
          showExtensionsList={showExtensionsList}
          showCommandsList={showCommandsList}
          showApplicationsList={showApplicationsList}
          filteredExtensions={filteredExtensions}
          extensionsLoaded={extensionsLoaded}
          mentionIndex={mentionIndex}
          onSelectExtension={selectExtension}
          filteredSlashCommands={filteredSlashCommands}
          slashIndex={slashIndex}
          onExecuteCommand={executeSlashCommand}
          filteredApplications={filteredApplications}
          applicationsLoaded={applicationsLoaded}
          applicationsLoading={applicationsLoading}
          applicationsCount={applicationsCount}
          applicationsError={applicationsError}
          applicationIndex={applicationIndex}
          starQuery={starState?.query ?? null}
          onOpenApplication={openApplicationById}
          turns={turns}
          error={error}
          composerActions={composerActions}
          scrollRef={scrollRef}
        />

        <FooterArea>
          <HintBar
            left={
              <>
                <HintItem label="Navigate / History" keyHint={<KeyHint keys={["↑", "↓"]} />} />
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
