import { useEffect } from "react";
import { useParams } from "react-router-dom";
import { closeExtensionWindow } from "../../../lib/ipc";
import { getExtensionView } from "../../extension/extension-views";
import { useExtensionContext } from "../../extension/extension.context";
import { useServerContext } from "../../server/server.context";
import { ExtensionSdkProvider } from "@cocommand/sdk/react";
import { Text } from "@cocommand/ui";

export function ExtensionWindowView() {
  const { extensionId } = useParams<{ extensionId: string }>();
  const dynamicViewsLoaded = useExtensionContext((s) => s.dynamicViewsLoaded);
  const fetchExtensions = useExtensionContext((s) => s.fetchExtensions);
  // Subscribe to viewLoadVersion so we re-render when dynamic views are loaded
  useExtensionContext((s) => s.viewLoadVersion);

  const serverInfo = useServerContext((s) => s.info);
  const addr = serverInfo?.addr ?? "";
  const baseUrl = addr.startsWith("http") ? addr : `http://${addr}`;

  // Popout windows are fresh page loads with an empty view registry.
  // Trigger fetchExtensions which in turn runs loadDynamicViews.
  useEffect(() => {
    if (!dynamicViewsLoaded) {
      fetchExtensions();
    }
  }, [dynamicViewsLoaded, fetchExtensions]);

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
    if (!dynamicViewsLoaded) {
      return <Text size="sm" tone="secondary">Loading extension view...</Text>;
    }
    return <Text size="sm" tone="secondary">No view available for extension "{extensionId}".</Text>;
  }

  const Component = config.component;
  return (
    <div className="app-shell" style={{ width: "100%", height: "100%", "--cc-extension-bg": "var(--cc-surface-primary)" } as React.CSSProperties}>
      <ExtensionSdkProvider baseUrl={baseUrl} extensionId={extensionId}>
        <Component mode="popout" />
      </ExtensionSdkProvider>
    </div>
  );
}
