import "@cocommand/ui";
import styles from "./UiKitView.module.css";
import {
  ActionHint,
  Chip,
  ChipGroup,
  CloseButton,
  CommandPaletteShell,
  ContentArea,
  Divider,
  FilterArea,
  FooterArea,
  HeaderArea,
  HintBar,
  HintItem,
  Icon,
  IconContainer,
  KeyHint,
  ListItem,
  ListSection,
  SearchField,
  Text,
} from "@cocommand/ui";

const SearchIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
    <circle cx="11" cy="11" r="6" />
    <path d="M20 20l-3.8-3.8" />
  </svg>
);

const ChartIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
    <path d="M4 19h16" />
    <path d="M7 15v-6" />
    <path d="M12 19v-10" />
    <path d="M17 19v-4" />
  </svg>
);

const ListIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
    <path d="M5 7h14" />
    <path d="M5 12h14" />
    <path d="M5 17h10" />
  </svg>
);

const ArrowIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
    <path d="M7 17l10-10" />
    <path d="M9 7h8v8" />
  </svg>
);

export function UiKitView() {
  return (
    <main className={`${styles.page} cc-theme-dark cc-reset`}>
      <CommandPaletteShell className={styles.shell}>
        <HeaderArea>
          <SearchField icon={<Icon>{SearchIcon}</Icon>} shortcut={["⌘", "F"]} />
          <Divider />
        </HeaderArea>

        <FilterArea>
          <ChipGroup>
            <Chip label="Templates" active icon={<Icon>{ListIcon}</Icon>} />
            <Chip label="Documents" icon={<Icon>{ListIcon}</Icon>} />
            <Chip label="Inbox" icon={<Icon>{ListIcon}</Icon>} />
            <Chip label="Smart planning" icon={<Icon>{ListIcon}</Icon>} />
          </ChipGroup>
        </FilterArea>

        <ContentArea>
          <ListSection label="Recent searches">
            <ListItem
              title="Total engagement for Microdose Instagram"
              subtitle="Show dedicated dashboard"
              icon={<IconContainer><Icon>{ChartIcon}</Icon></IconContainer>}
              rightMeta={<KeyHint keys={["⌘", "1"]} />}
            />
            <ListItem
              selected
              title="Audience Growth"
              subtitle="See how your audience grew during the reporting period"
              icon={<IconContainer><Icon>{ChartIcon}</Icon></IconContainer>}
              rightMeta={<ActionHint label="Enter" icon={<Icon>{ArrowIcon}</Icon>} />}
            />
            <ListItem
              title="Total impressions overview"
              subtitle="Discover the impressions data of your audience."
              icon={<IconContainer><Icon>{ChartIcon}</Icon></IconContainer>}
              rightMeta={<KeyHint keys={["⌘", "3"]} />}
            />
          </ListSection>

          <Divider />

          <ListSection label="Jump to">
            <ListItem
              title="New Report"
              icon={<IconContainer><Icon>{ListIcon}</Icon></IconContainer>}
              rightMeta={<Text size="xs" tone="tertiary">Press</Text>}
            />
            <ListItem
              title="Campaigns"
              subtitle="See how your campaigns performing"
              icon={<IconContainer><Icon>{ListIcon}</Icon></IconContainer>}
              rightMeta={<KeyHint keys={["⌘", "5"]} />}
            />
          </ListSection>
        </ContentArea>

        <FooterArea>
          <HintBar
            left={
              <>
                <HintItem label="Navigate" keyHint={<KeyHint keys={["↑", "↓"]} />} />
                <HintItem label="Enter" keyHint={<KeyHint keys="↵" />} />
                <HintItem label="Command" keyHint={<KeyHint keys="/" />} />
                <HintItem label="Guide" keyHint={<KeyHint keys="?" />} />
              </>
            }
            right={<CloseButton />}
          />
        </FooterArea>
      </CommandPaletteShell>
    </main>
  );
}
