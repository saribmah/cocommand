import { useEffect } from "react";
import { useParams } from "react-router-dom";
import { closeExtensionWindow } from "../../../lib/ipc";
import { getExtensionView } from "../../extension/extension-views";
import { Text } from "@cocommand/ui";

export function ExtensionWindowView() {
  const { extensionId } = useParams<{ extensionId: string }>();

  useEffect(() => {
    if (!extensionId) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeExtensionWindow(extensionId);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [extensionId]);

  if (!extensionId) {
    return <Text size="sm" tone="secondary">Missing extension ID.</Text>;
  }

  const config = getExtensionView(extensionId);

  if (!config) {
    return <Text size="sm" tone="secondary">No view available for extension "{extensionId}".</Text>;
  }

  const Component = config.component;
  return (
    <div className="app-shell" style={{ width: "100%", height: "100%", "--cc-extension-bg": "var(--cc-surface-primary)" } as React.CSSProperties}>
      <Component mode="popout" />
    </div>
  );
}
