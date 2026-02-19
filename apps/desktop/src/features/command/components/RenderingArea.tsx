import type { RefObject } from "react";
import {
  ActionHint,
  ArrowIcon,
  CommandIcon,
  ContentArea,
  Divider,
  ErrorCard,
  ExtensionIcon,
  FileCard,
  Icon,
  IconContainer,
  ListItem,
  ListSection,
  MarkdownResponseCard,
  ReasoningCard,
  ResponseStack,
  Text,
  ToolCallCard,
} from "@cocommand/ui";
import type { ExtensionInfo } from "../../extension/extension.types";
import type { ApplicationInfo } from "../../application/application.types";
import type { MessagePart } from "../command.types";
import type { ComposerActions } from "../composer-actions";
import {
  formatFileType,
  getToolError,
  getToolParams,
  getToolResult,
  mapToolStateToCard,
} from "../composer-utils";
import { ExtensionViewContainer } from "./ExtensionViewContainer";
import styles from "../command.module.css";

const ApplicationIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="3" y="4" width="18" height="14" rx="2" />
    <path d="M8 20h8" />
    <path d="M12 18v2" />
  </svg>
);

interface SlashCommand {
  id: string;
  name: string;
  description: string;
}

interface RenderingAreaProps {
  // View routing
  showExtensionView: boolean;
  activeExtensionId: string | null;
  showExtensionsList: boolean;
  showCommandsList: boolean;
  showApplicationsList: boolean;

  // Extension list
  filteredExtensions: ExtensionInfo[];
  extensionsLoaded: boolean;
  mentionIndex: number;
  onSelectExtension: (ext: { id: string; name: string; kind: string }) => void;

  // Command list
  filteredSlashCommands: SlashCommand[];
  slashIndex: number;
  onExecuteCommand: (id: string) => void;

  // Application list
  filteredApplications: ApplicationInfo[];
  applicationsLoaded: boolean;
  applicationsLoading: boolean;
  applicationsCount: number;
  applicationsError: string | null;
  applicationIndex: number;
  starQuery: string | null;
  onOpenApplication: (app: ApplicationInfo) => void;

  // Response
  parts: MessagePart[];
  error: string | null;

  // Extension view support
  composerActions: ComposerActions;

  // Scroll
  scrollRef: RefObject<HTMLDivElement | null>;
}

export function RenderingArea({
  showExtensionView,
  activeExtensionId,
  showExtensionsList,
  showCommandsList,
  showApplicationsList,
  filteredExtensions,
  extensionsLoaded,
  mentionIndex,
  onSelectExtension,
  filteredSlashCommands,
  slashIndex,
  onExecuteCommand,
  filteredApplications,
  applicationsLoaded,
  applicationsLoading,
  applicationsCount,
  applicationsError,
  applicationIndex,
  starQuery,
  onOpenApplication,
  parts,
  error,
  composerActions,
  scrollRef,
}: RenderingAreaProps) {
  const showResponses = parts.length > 0 || !!error;

  return (
    <ContentArea className={styles.content}>
      {showExtensionView && activeExtensionId ? (
        <ExtensionViewContainer
          extensionId={activeExtensionId}
          actions={composerActions}
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
                      onSelectExtension({
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
                    onExecuteCommand(command.id);
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
                      onOpenApplication(application);
                    }}
                  />
                ))
              ) : starQuery ? (
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
  );
}
