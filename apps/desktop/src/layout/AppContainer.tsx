import type { ReactNode } from "react";

interface AppContainerProps {
  children: ReactNode;
  className?: string;
}

export function AppContainer({ children, className }: AppContainerProps) {
  return (
    <div
      className={["cc-theme-dark", "cc-reset", "app-container", className]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </div>
  );
}
