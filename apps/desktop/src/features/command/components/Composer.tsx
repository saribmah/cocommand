import type { KeyboardEvent, RefObject } from "react";
import {
  Divider,
  ExtensionIcon,
  HeaderArea,
  Icon,
  SearchField,
  SearchIcon,
  StatusBadge,
} from "@cocommand/ui";
import type { ComposerTagSegment } from "../composer-utils";
import styles from "../command.module.css";

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

interface ComposerProps {
  inputId: string;
  inputRef: RefObject<HTMLInputElement | null>;
  activeText: string;
  tagSegments: ComposerTagSegment[];
  isSubmitting: boolean;
  serverOnline: boolean;
  placeholder: string;
  onTextChange: (value: string) => void;
  onKeyDown: (e: KeyboardEvent<HTMLInputElement>) => void;
  onRemoveSegment: (segment: ComposerTagSegment) => void;
  onClickSegment: (segment: ComposerTagSegment) => void;
}

export function Composer({
  inputId,
  inputRef,
  activeText,
  tagSegments,
  isSubmitting,
  serverOnline,
  placeholder,
  onTextChange,
  onKeyDown,
  onRemoveSegment,
  onClickSegment,
}: ComposerProps) {
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
                onClick={() => onClickSegment(segment)}
                style={{ cursor: "pointer" }}
              >
                <Icon size={14}>{ExtensionIcon}</Icon>
                <span className={styles.targetTagLabel}>@{segment.part.name}</span>
                <button
                  type="button"
                  className={styles.targetTagRemove}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => onRemoveSegment(segment)}
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
                onClick={() => onRemoveSegment(segment)}
                aria-label={`Remove ${segment.part.name}`}
              >
                <Icon size={12}>{RemoveIcon}</Icon>
              </button>
            </span>
          );
        })}
      </div>
    ) : undefined;

  return (
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
            onChange: (e) => onTextChange(e.target.value),
            onKeyDown,
            disabled: isSubmitting,
            spellCheck: false,
            autoComplete: "off",
          }}
        />
        <StatusBadge
          status={serverOnline ? "good" : "warn"}
          label={serverOnline ? "online" : "offline"}
        />
      </div>
      <Divider />
    </HeaderArea>
  );
}
