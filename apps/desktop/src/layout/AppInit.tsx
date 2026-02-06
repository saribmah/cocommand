import {ReactNode, useCallback, useEffect, useState} from "react";
import {AppPanel, ButtonPrimary, Text} from "@cocommand/ui";
import {ExtensionProvider} from "../features/extension/extension.provider";
import {OnboardingProvider} from "../features/onboarding/onboarding.provider";
import {ServerProvider} from "../features/server/server.provider.tsx";
import {SessionProvider} from "../features/session/session.provider";
import {WorkspaceProvider} from "../features/workspace/workspace.provider";
import {getServerStatus, ServerStatus} from "../lib/ipc.ts";

interface AppInitProps {
    children: ReactNode;
}

export function AppInit({ children }: AppInitProps) {
    const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);

    // Check sandbox health
    const fetchServerStatus = useCallback(async (): Promise<void> => {
        try {
            const status = await getServerStatus();
            setServerStatus(status)
        }
        catch (error) {
            console.error('Failed to check sandbox health:', error);
        }
    }, []);

    useEffect(() => {
        fetchServerStatus();
    }, [fetchServerStatus]);

    useEffect(() => {
        if (serverStatus?.status === "ready" || serverStatus?.status === "error") return;
        const timer = window.setInterval(() => {
            fetchServerStatus();
        }, 500);
        return () => window.clearInterval(timer);
    }, [serverStatus, fetchServerStatus]);

    if (!serverStatus || serverStatus?.status !== "ready") {
        return (
            <AppPanel style={{ minHeight: 360, maxWidth: 620 }}>
                <Text as="div" size="lg" weight="semibold">
                    {serverStatus?.status === "starting"
                        ? "Starting Cocommand server..."
                        : "Failed to start server"}
                </Text>
                <Text as="div" size="sm" tone="secondary">
                    {serverStatus?.status === "starting"
                        ? "Waiting for backend startup."
                        : serverStatus?.error ?? "Unknown error."}
                </Text>
                {serverStatus?.status === "error" ? (
                    <ButtonPrimary onClick={() => fetchServerStatus()}>
                        Retry
                    </ButtonPrimary>
                ) : null}
            </AppPanel>
        );
    }

    return (
        <ServerProvider status={serverStatus}>
            <WorkspaceProvider>
                <OnboardingProvider>
                    <SessionProvider>
                        <ExtensionProvider>
                            {children}
                        </ExtensionProvider>
                    </SessionProvider>
                </OnboardingProvider>
            </WorkspaceProvider>
        </ServerProvider>
    );
}
