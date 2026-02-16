import { getExtensionView } from "../extension-views";
import { openExtensionWindow } from "../../../lib/ipc";
import { Text } from "@cocommand/ui";
import styles from "../command.module.css";

interface ExtensionViewContainerProps {
  extensionId: string;
  extraProps?: Record<string, unknown>;
}

const PopoutIcon = (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
    <polyline points="15 3 21 3 21 9" />
    <line x1="10" y1="14" x2="21" y2="3" />
  </svg>
);

export function ExtensionViewContainer({ extensionId, extraProps }: ExtensionViewContainerProps) {
  const config = getExtensionView(extensionId);

  if (!config) {
    return (
      <Text size="sm" tone="secondary">
        No view available for this extension.
      </Text>
    );
  }

  const Component = config.component;

  const handlePopout = () => {
    const popout = config.popout ?? { width: 700, height: 500, title: config.label };
    openExtensionWindow({
      extensionId,
      title: popout.title,
      width: popout.width,
      height: popout.height,
    }).catch((err) => {
      console.error("Failed to open extension window:", err);
    });
  };

  return (
    <div className={styles.extensionViewContainer}>
      <div className={styles.extensionViewHeader}>
        <button
          type="button"
          className={styles.extensionViewPopout}
          onClick={handlePopout}
          title={`Open ${config.label} in a new window`}
        >
          {PopoutIcon}
        </button>
      </div>
      <div className={styles.extensionViewContent}>
        <Component mode="inline" {...extraProps} />
      </div>
    </div>
  );
}
