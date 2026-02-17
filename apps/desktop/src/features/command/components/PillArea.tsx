import { Badge, Chip, ChipGroup, FilterArea } from "@cocommand/ui";
import type { FilterTab } from "../composer-utils";
import styles from "../command.module.css";

interface PillAreaProps {
  activeView: FilterTab;
  isSubmitting: boolean;
  extensionPills: Array<{ extensionId: string; name: string }>;
  onTabChange: (tab: FilterTab) => void;
  onExtensionsClick: () => void;
  onCommandsClick: () => void;
  onApplicationsClick: () => void;
}

export function PillArea({
  activeView,
  isSubmitting,
  extensionPills,
  onTabChange,
  onExtensionsClick,
  onCommandsClick,
  onApplicationsClick,
}: PillAreaProps) {
  return (
    <FilterArea>
      <div className={styles.filterRow}>
        <ChipGroup>
          <Chip
            label="Recent"
            active={activeView === "recent"}
            onClick={() => onTabChange("recent")}
          />
          <Chip
            label="Extensions"
            active={activeView === "extensions"}
            onClick={onExtensionsClick}
          />
          <Chip
            label="Commands"
            active={activeView === "commands"}
            onClick={onCommandsClick}
          />
          <Chip
            label="Applications"
            active={activeView === "applications"}
            onClick={onApplicationsClick}
          />
          {extensionPills.map((pill) => (
            <Chip
              key={`ext-pill-${pill.extensionId}`}
              label={pill.name}
              active={activeView === `ext:${pill.extensionId}`}
              onClick={() => onTabChange(`ext:${pill.extensionId}`)}
            />
          ))}
        </ChipGroup>
        {isSubmitting ? <Badge>Working...</Badge> : null}
      </div>
    </FilterArea>
  );
}
