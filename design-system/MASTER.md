# Meetral — Design System (Master)

> **Audience:** AI Agents implementing or reviewing UI  
> **Hierarchy:** Check `design-system/pages/[screen].md` first when editing a specific screen. Page rules **override** this file.

**Project:** Meetral (Tauri desktop)  
**Last updated:** 2026-07-15  
**Stack:** React 19 · Vite 6 · Tauri 2 · Tailwind CSS v4 · shadcn/ui (New York) · Lucide React  
**Theme:** Dual theme (Dark OLED default · Light opt-in) · Flat · Minimal · Desktop-first (min width ~720px)

---

## 0. Agent quick start

Before any UI work:

1. Read this file + the relevant page override in `design-system/pages/`.
2. Inspect `src/index.css` for live CSS variable values.
3. Reuse `src/shared/ui/*` (shadcn) and `src/shared/components/*` (app wrappers).
4. Feature UI lives under `src/features/{pipeline,config,ai,voice,audio,meeting,overlay}/`.
5. **Do not** apply generic AI palettes — this app has a fixed OLED theme.
6. **Do not** reintroduce `App.css`, `src/components/ui/`, or legacy `.btn-*` classes.
7. Run `npm run build && npm test` before finishing.

---

## 1. Principles

1. **One visual language** — Live, History drawer, Settings drawer, Meeting Detail share the same tokens and shadcn primitives.
2. **Content over chrome** — Minimize header height; surfaces carry content.
3. **Semantic color** — Color communicates status (live, ready, warning, error); never decorative-only.
4. **Recovery paths** — Empty states and errors include a clear next action.
5. **No layout-shift motion** — Hover/focus via color, border, opacity only (no `scale` on buttons/cards).
6. **Lucide only** — No emoji icons; no unicode arrows (`⇄`, `◄►`, `↗`) in new UI.
7. **Tailwind + tokens** — Prefer `bg-background`, `text-muted-foreground`, `border-border`; avoid new random hex except domain accents documented below.

---

## 2. Color system

App supports **Dark** (default OLED) and **Light**. Preference: Settings → App → Theme (`dark` | `light` | `system`). `<html class="dark">` when resolved theme is dark.

### shadcn CSS variables (`src/index.css`)

| Variable | Light | Dark | Role |
|----------|-------|------|------|
| `--background` | `#f4f6f8` | `#0f1117` | App shell, body |
| `--foreground` | `#1a1d24` | `#e8eaed` | Primary text |
| `--card` | `#ffffff` | `#14171d` | Panes, surfaces |
| `--secondary` / `--muted` | `#dde3ea` | `#1e2430` | Elevated rows, inactive tabs |
| `--border` / `--input` | `#c5ced8` | `#2a2f3a` | Default borders, inputs |
| `--primary` / `--ring` | `#0ea5c6` | `#148a9c` / `#5bc4f0` | CTA + focus (light = sky cyan; dark = teal primary + sky ring) |
| `--accent` | `#11A9EE` | `#5bc4f0` | Brand sky (light) / sky accent (dark) |
| `--destructive` | `#d93025` | `#f28b82` | Errors, destructive text |
| `--success` | `#1e8e3e` | `#81c995` | Live / Ready status |
| `--warning` | `#e37400` | `#fbbc04` | Setup / attention |
| `--warning-bg` | `#fff3e0` | `#3c2f10` | Warning chip fills |
| `--hover-surface` | `#d0d8e2` | `#252b38` | Hover backgrounds |
| `--muted-foreground` | `#4a5560` | `#9aa0a6` | Secondary text |
| `--radius` | `0.5rem` (8px) | same | Default shadcn radius |

**Light contrast:** Prefer `bg-secondary border-border` for mode segment tracks (and `bg-card border-border` for other chrome). Status chips use `bg-[var(--warning-bg)]` / `--pipeline-*` — never OLED hex (`#141820`, `#3c2f10`, `#c4c9d0`).

### Legacy tokens (still in `:root`)

Use shadcn variables in new code. Legacy aliases (`--bg-base`, `--cta`, `--danger`, …) exist for gradual cleanup only.

### Domain-specific accents (pipeline toolbar)

Direct/Translate mode controls use custom colors in `src/features/pipeline/lib/pipelineColors.ts` and shared Lucide icons in `pipelineModeIcons.ts` (`Zap` / `Languages` + output mode icons). Live path segment is a **compact floating toggle on the single-row column toolbar** (`h-8`, `w-max max-w-[17rem]`), not a full-bleed second row. **Same elevation recipe light/dark** via `--pipeline-path-*`: recessed track + active pill. Active fill is the **domain wash** (`--pipeline-direct-bg` / `--pipeline-translate-bg`) with **hover-level shadow by default** so active always outranks idle. Idle: `text-foreground/55`; hover only brightens text / faint wash — **no** white elevated pill (that competed with active).

| Mode | Active styling |
|------|----------------|
| Direct | Domain green wash + `var(--pipeline-direct-text)` + strong pill shadow |
| Translate | Domain cyan wash + `var(--pipeline-translate-text)` + strong pill shadow |

`--pipeline-*-bg` / hover still used by status chips and overlay chrome, not the Live mode segment. Live attention (`ColumnHeaderStatus`) is icon-only `size-6`; **ready is hidden**.

Header session hub (`SessionHeaderChip`) uses `src/shared/lib/headerChrome.ts` — `h-8`, `rounded-md`, cells: **meeting elapsed** (`w-14`, `useMeetingElapsed` from `MeetingRecord.startedAtMs`) · languages · context (`w-14` Sparkles). Side cells share width for symmetry; no live Radio glyph. Live column mute is `size-8`; column status is icon-only `size-6` (no label chip). Per-column WS session elapsed is not shown on Live/Overlay chrome.

Do not generalize pipeline accents into global tokens unless extracting a shared constant.

### Citation chips (`AnchorTimeBadge`)

Compact transcript-segment citations (`m:ss` + direction dot) in **TipTap summary** and **Artifacts** — implemented via `AnchorTimeBadge` / `anchorBadgeStyle.ts`. Not a chat surface; chips jump to transcript time on click. See [pages/meeting-detail.md](pages/meeting-detail.md).

### Semantic usage

| Meaning | Implementation |
|---------|----------------|
| Primary action | `Button variant="default"` |
| Secondary action | `Button variant="secondary"` |
| Tertiary / link | `Button variant="ghost"` or `variant="link"` |
| Destructive | `AppButton destructive` — **never** solid red as default delete |
| Live / active | `AppBadge tone="ok"` or `text-success` |
| Setup / attention | `AppBadge tone="warn"`, setup dot on Settings icon |
| Error | `role="alert"` + destructive styling, or `Alert variant="destructive"` |

---

## 3. Typography

### Font stack

```css
font-family: "Segoe UI", system-ui, -apple-system, sans-serif;
```

### Tailwind scale

| Role | Classes | Usage |
|------|---------|-------|
| Section label | `text-xs font-semibold uppercase tracking-wider text-muted-foreground` | `SectionHeading` — Library groups; Settings **group** titles |
| Body | `text-sm` (14px) | Default UI copy |
| Column header | `text-sm font-semibold` | Live + detail columns |
| Page title | `text-base font-semibold` or `text-lg` | Meeting detail `h1`; Settings sheet title |
| Micro badge | `text-xs` | Tab counts, pipeline badges |

Always use `SectionHeading` for uppercase section labels — do not recreate ad-hoc labels.

**Settings hierarchy** (drawer only — see [pages/settings.md](pages/settings.md)):

| Level | Classes | Component |
|-------|---------|-----------|
| L0 Sheet title | `text-base font-semibold text-foreground` | `SettingsDrawer` `h2` |
| L1 Section | `text-sm font-semibold tracking-tight text-foreground` | `SettingsSection` `h3` |
| L2 Group | `SectionHeading` (uppercase muted) | `SettingsGroup` |
| L3 Field | `settingsFieldLabelClass` (`text-sm font-medium text-muted-foreground`) | `Label` in Settings |
| L4 Description | `text-xs text-muted-foreground` | group/field hints |

---

## 4. Spacing & layout

### App shell (`src/shared/layout/AppShell.tsx`)

```
flex h-full min-h-0 flex-col gap-0 p-3
  header: grid … pb-3 (no border-b)
  main: flex min-h-0 flex-1 flex-col pt-3
```

| Region | Spec |
|--------|------|
| Outer padding | `p-3` (12px) |
| Header grid | 3 columns: start \| center \| end; spacing only (no chrome hairline) |
| Main | `flex-1 min-h-0 pt-3` — scroll inside child panes |

### Header by view

| View | Start | Center | End |
|------|-------|--------|-----|
| Live | `HistoryButton` | `SessionHeaderHub` (elapsed · languages · context) | Settings |
| Meeting detail | Back **Live** (`ArrowLeft`) | `MeetingDetailHeaderTitle` (title + meta chip) | Settings |
| Loading | `LoadingSkeleton` mirrors same header grid + dual panes | | |

Settings attention: warning dot on Settings icon when setup incomplete.

### Drawers (shadcn `Sheet`)

| Drawer | Side | Width | Component |
|--------|------|-------|-----------|
| History | Left | `320px` | `LibraryDrawer` |
| Settings | Right | `440px` | `SettingsDrawer` |

Shared pattern: `Sheet` + `SheetContent` with `showCloseButton={false}`; compact header + ghost `X`; body scrolls; overlay/Escape closes.

### Content surfaces

```
rounded-[10px] bg-card
```

Live and meeting-detail columns: elevated panes via `--pane-elevated` / `pipelineToolbarClasses.paneSurface` (same shadow recipe light+dark, no outer border), toolbar rail `bg-secondary`, mid-gutter grip. Mode active pill and mute chrome use `bg-card` + border/ring in both themes (Live only).

### Resizable split

Live transcript uses `react-resizable-panels` via `TranscriptSplit` (You | Meeting). Meeting Detail uses a second horizontal split: Summary | `ReadonlyTranscriptTimeline` (`autoSaveId="meeting-detail-split-v1"`, default ~38% / ~62%).
---

## 5. Component inventory

### shadcn primitives (`src/shared/ui/`)

| Component | Primary use |
|-----------|-------------|
| `button` | All click actions |
| `badge` | Status, counts |
| `alert` | Banners |
| `sheet` | History + Settings drawers |
| `dialog` / `alert-dialog` | Modals, confirms |
| `tabs` | Settings tabs |
| `dropdown-menu` | Pipeline output, meeting actions |
| `select` | Language + device + provider pickers |
| `input` / `textarea` / `label` | Forms |
| `collapsible` | Settings advanced blocks |
| `checkbox` | Preferences |
| `scroll-area` | Setup guide modal |
| `tooltip` | Hover tips via `AppTooltip` / `SettingInfoHint` (do not use native `title` for chrome) |
| `skeleton` | Loading skeleton |
| `separator` | Visual dividers |
| `button-group` | Direct \| Translate; Segment layout |

Add via: `npx shadcn@latest add <name>` — config in `components.json` (aliases → `@/shared/*`).

### App shared wrappers (`src/shared/components/`, `src/shared/ui/`)

| Component | Purpose |
|-----------|---------|
| `AppToastHost` | First-party toast host (via `useToast()` / `ToastProvider`) |
| `SectionHeading` | Uppercase muted labels (Library groups; Settings L2 groups) |
| `AppBadge` | Section status: icon-only chip (`tone="ok" \| "warn" \| "muted"`); hover tooltip shows detail label |
| `AppButton` | `destructive` prop → outline + danger colors |
| `EmptyState` | Tier A empty regions |
| `InlineError` | Field-level red text |
| `LiveNotice` | Live notices: `.Strip` (global) + `.Rail` (column); shared tones/actions |
| `ConfirmDialog` | Destructive confirms |
| `SettingInfoHint` | Info-icon help; uses `AppTooltip` |
| `AppTooltip` | Surface tip matching popover/dropdown (`bg-popover`, `border-border`, fade only; open delay **500ms**, no skip between tips). Prefer over native `title`. |
| `SettingsGroup` | Settings L2 sub-block (no border); title via `SectionHeading` |
| `SecretApiKeyField` | Masked API key row: save, test, remove |
| `ApiKeyChip` | Settings header chip when key Ready — expands manage UI (Translate Engine, Summaries, ElevenLabs) |

### Domain components (do not duplicate patterns)

| Area | Key files under `src/features/` |
|------|----------------------------------|
| Live | `pipeline/…` — `TranslateView`, `TranscriptPanel`, `ColumnPipelineToolbar`, `SetupBanner`, `PipelineErrorBanner` |
| History | `meeting/library/…` — `LibraryDrawer`, `LibrarySearch`, `LibraryTree`, `LibrarySortableFolderList` |
| Settings | `config/…` + `ai/…` + `voice/…` + `audio/…` — `SettingsDrawer`, `SettingsSection`, … |
| Detail | `meeting/detail/…` — `MeetingDetailView`, `SummaryPanel`, `ReadonlyTranscriptTimeline` |
| Chrome | `HistoryButton`, `SessionHeaderHub`, `LoadingSkeleton` |

---

## 6. Buttons

Use shadcn `Button` from `@/shared/ui/button`.

| Intent | Variant / size |
|--------|----------------|
| Primary | `variant="default"` |
| Secondary | `variant="secondary"` |
| Ghost / tertiary | `variant="ghost"` or `link` |
| Destructive outline | `AppButton destructive` |
| Icon 36px | `size="icon"` |
| Compact | `size="sm"` / `icon-sm` / `icon-xs` |

Interaction: focus-visible ring; disabled opacity; `cursor-pointer` on enabled buttons (global in `index.css`).

---

## 7. Feedback & toasts

**Single source:** `useToast()` from `@/shared/context/toastContext` (first-party `AppToastHost`).

| Type | When |
|------|------|
| `useToast()` | Save, rename, reconnect |
| Inline `role="alert"` / `Alert` | Fatal pipeline error (non-Live surfaces) |
| `LiveNotice.Strip` / `.Rail` | Shared Live notice shell (strip under chrome · rail in column) |
| `PipelineErrorBanner` | Live: device-unavailable or fatal → `LiveNotice.Strip` |
| `SetupBanner` | Live: incomplete setup → `LiveNotice.Strip` |
| Column adapters | `AudioDeviceBanner` / `ConnectionBanner` / `SessionDriftHint` → `LiveNotice.Rail` |
| Field error | Settings validation |
| `ConfirmDialog` / `InlineConfirm` | Delete, discard, end meeting |

Never full-page error for partial failure. Never local toast components.

---

## 8. Empty states

| Tier | Pattern |
|------|---------|
| A | `EmptyState` — icon + title + description + action |
| B | `text-sm text-muted-foreground` inline |
| C | Meeting detail tab dots/counts via `TabIndicator` |

---

## 9. Icons

- Lucide React only
- Sizes: 14 chip, 16 button, 20 header, 40 empty-state
- Icon-only buttons: `aria-label` required
- Flags: `FlagIcon` (exception)

---

## 10. Motion & accessibility

- Prefer color/opacity transitions 150–200ms; respect `prefers-reduced-motion`
- Focus — two tiers, never bare `outline: none`:
  - **Quiet border** (`focus-visible:border-ring`): text-entry controls — `Input`, `Textarea`, `Select` triggers — plus dense ambient surfaces that use `focus-within:border-ring/60` on a container card. Rationale: these controls match `:focus-visible` on mouse click, so a 3px halo reads as visual noise during normal pointer use.
  - **Standard ring** (shadcn `focus-visible:ring-[3px] ring-ring/50`): `Button`, `Checkbox`, `Tabs`, `Badge`, keyboard-nav rows. Their halo only appears on keyboard focus — exactly when wayfinding matters most. Do not quiet these.
- Drawers: Radix focus trap via `Sheet`
- Test widths: **720px** (min) and **984px** (default)

---

## 11. Screen map

```
App
├── Live — appView === "live"
│   ├── Header: History | Lang chip | [Soniox context] Settings
│   ├── PipelineErrorBanner / SetupBanner → LiveNotice.Strip (conditional)
│   └── TranscriptPanel → TranscriptSplit (You | Meeting)
├── History — libraryOpen (Sheet left)
│   ├── Current session (if live) · Recent · Folders
│   └── Footer: New meeting
├── Settings — drawerOpen (Sheet right)
│   └── Tabs: Translate | Voice | Audio | Intelligence | App
└── Meeting Detail — appView === "meeting-detail"
    ├── Header: Live back | Settings
    ├── Title + subtitle + ⋯ menu
    └── Tabs: Summary | Transcript
```

### Page overrides

| Screen | File |
|--------|------|
| Live | `design-system/pages/live.md` |
| History | `design-system/pages/history.md` |
| Settings | `design-system/pages/settings.md` |
| Overlay | `design-system/pages/overlay.md` |
| Meeting detail | `design-system/pages/meeting-detail.md` |

---

## 12. Anti-patterns

- ❌ Generic ui-ux-pro-max indigo/emerald palette (use Meetral teal/sky tokens)
- ❌ Emojis as icons; unicode arrows in new UI
- ❌ Paths `src/components/ui/` or `src/components/shared/` (removed — use `src/shared/`)
- ❌ Legacy `.btn-*`, `App.css`, `DismissButton`, `StatusChip`
- ❌ Local toast instances — use `useToast()`
- ❌ Full-page error for partial action failure
- ❌ Hover `scale` that shifts layout
- ❌ New random hex without documenting in this file
- ❌ `<div role="button">` — use shadcn `Button`
- ❌ Single long Settings scroll — keep the five-tab IA

---

## 13. Pre-delivery checklist

- [ ] Semantic Tailwind / CSS variables (no stray hex except documented accents)
- [ ] `Button` variant from §6
- [ ] Lucide icons with correct `aria-*`
- [ ] Focus ring visible; empty/error recovery paths
- [ ] Tested at 720px and 984px
- [ ] `npm run build && npm test` pass

---

## 14. References

| Resource | Path |
|----------|------|
| Global styles | `src/index.css` |
| shadcn config | `components.json` |
| cn() helper | `src/shared/lib/utils.ts` |
| Pipeline accents | `src/features/pipeline/lib/pipelineColors.ts` |
| Architecture | `docs/architecture/overview.md` |
| Agent router | `AGENTS.md` |
