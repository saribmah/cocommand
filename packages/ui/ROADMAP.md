# UI Roadmap

This roadmap is derived from `mocks/mock-1.webp` and `mocks/mock-2.webp`.

## Phase 1: Primitives (Atoms)
- PanelSurface
- Divider
- Text (variants: primary/secondary/tertiary; sizes: xs/sm/md/lg)
- Icon (line icon wrapper)
- IconContainer (rounded square backing)
- Pill (small rounded capsule)
- KeyHint (pill with key text)

## Phase 2: Controls (Molecules)
- SearchField (icon + placeholder + optional shortcut)
- Chip
- ChipGroup
- ActionHint (text + icon, e.g., “Enter ↗”)

## Phase 3: List Items
- ListItem (icon + title + subtitle + right meta)
- ListSectionHeader (label)
- ListSection (header + list)

## Phase 4: Footer / Help Bar
- HintBar
- HintItem (key + label)
- CloseButton (esc pill + label)

## Phase 5: Layout Regions
- CommandPaletteShell
- HeaderArea (SearchField + Divider)
- FilterArea (ChipGroup)
- ContentArea (ListSections)
- FooterArea (HintBar)

## Onboarding + Settings (Unified UX)

### Shared Layout & Navigation
- AppShell (window background + centered layout)
- AppPanel (frosted container)
- AppHeader (brand + title + subtitle)
- AppNav (nav row container)
- AppContent (scrollable content area)
- AppFooter (actions footer)
- NavTabs (tab/step container)
- NavTab (states: default/active/done/disabled)
- NavProgress (optional progress underline)

### Form System
- Field (label + control + help + error)
- FieldRow (input + action button)
- TextInput
- TextArea
- InlineHelp
- ErrorBanner
- ButtonPrimary
- ButtonSecondary
- ButtonGroup

### Selection Controls
- ChoiceCard (default/selected/disabled)
- AccentSwatch (color chip + label)
- OptionGroup (row layout)

### Status & Info
- StatusBadge (Granted/Not granted)
- InfoCard
- HighlightGrid
- HighlightItem
