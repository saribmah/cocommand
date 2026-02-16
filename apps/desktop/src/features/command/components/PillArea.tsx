import { Badge, Chip, ChipGroup, FilterArea } from "@cocommand/ui";
import type { FilterTab } from "../composer-utils";
import styles from "../command.module.css";

interface PillAreaProps {
  activeTab: FilterTab;
  isSubmitting: boolean;
  mentionActive: boolean;
  slashActive: boolean;
  hashActive: boolean;
  starActive: boolean;
  extensionPills: Array<{ extensionId: string; name: string }>;
  onTabChange: (tab: FilterTab) => void;
  onExtensionsClick: () => void;
  onCommandsClick: () => void;
  onApplicationsClick: () => void;
}

export function PillArea({
  activeTab,
  isSubmitting,
  mentionActive,
  slashActive,
  hashActive,
  starActive,
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
            active={
              activeTab === "recent" &&
              !mentionActive &&
              !slashActive &&
              !hashActive &&
              !starActive
            }
            onClick={() => onTabChange("recent")}
          />
          <Chip
            label="Extensions"
            active={activeTab === "extensions" || mentionActive}
            onClick={onExtensionsClick}
          />
          <Chip
            label="Commands"
            active={activeTab === "commands" || (slashActive && !mentionActive)}
            onClick={onCommandsClick}
          />
          <Chip
            label="Applications"
            active={activeTab === "applications" || starActive}
            onClick={onApplicationsClick}
          />
          {extensionPills.map((pill) => (
            <Chip
              key={`ext-pill-${pill.extensionId}`}
              label={pill.name}
              active={activeTab === `ext:${pill.extensionId}`}
              onClick={() => onTabChange(`ext:${pill.extensionId}`)}
            />
          ))}
        </ChipGroup>
        {isSubmitting ? <Badge>Working...</Badge> : null}
      </div>
    </FilterArea>
  );
}
