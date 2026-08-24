# Page override: Live (Translate View)

> Overrides `design-system/MASTER.md` where noted below.

**State:** `appView === "live"`
**Root:** `src/features/pipeline/components/TranslateView.tsx`
**Related:** `TranscriptPanel`, `TranscriptSplit`, `ColumnPipelineToolbar`, `LiveTranscriptColumn`, `LiveNotice`, `PipelineErrorBanner`, `SetupBanner`, `SessionHeaderHub`

---

## Layout

```
┌─────────────────────────────────────────────────┐
│ [History]     │ 0:12 | flags | ⌶ │      [⚙] │  ← chrome strip
├─────────────────────────────────────────────────┤
│ [PipelineErrorBanner / SetupBanner → LiveNotice.Strip] │
├─ pane card ───────────┬─ pane card ─────────────┤
│ toolbar rail          │ toolbar rail            │
│ transcript            │ transcript              │
└───────────────────────┴─────────────────────────┘
```

- Main: `TranslateView` → `TranscriptPanel` → `TranscriptSplit` (horizontal resizable).
- **Spacing:** `AppShell` column `gap-2` (header ↔ content); Live banners + panes use the same `gap-2` in `TranslateView` (no stacked `pb`/`pt`/`mb` on the strip).
- **Surface hierarchy (Live):** shell `bg-background` · header + panes separated by spacing only (no header `border-b`) · each column elevated pane (`paneSurface` / `--pane-elevated`, same recipe light+dark) · toolbar rail opaque `bg-secondary` · mid-gutter `GripVertical` resize.
- Mode segment active: `bg-card` + `ring-border/70` + `shadow-sm` in **both** themes (no dark-only hover-surface swap).
- Mute buttons: `bg-card border-border` both themes (override outline `dark:bg-input/30`).
- Live speech presence: `border-l-accent` on the live transcript row only (not the pane shell — avoids double left accent).
- **Session hub** (header center): `SessionHeaderHub` — one-line chip (`h-8`): **meeting elapsed** (`w-14`, `tabular-nums` from `meeting.startedAtMs`) · languages (flags + `ArrowLeftRight`) · context (`w-14` `Sparkles`, Soniox). Equal side cells for balance. Title **not** on chip (tooltip + popover only). Click opens popover (not Settings).
- Meeting elapsed: frontend `now - startedAtMs` via `useMeetingElapsed` (shared clock); independent of per-column WS `activeSince`. **Hidden when no live meeting** (no empty left spacer).
- No live `Radio` indicator on the chip (meeting presence = elapsed + hub itself).
- Popover: rename / end meeting (confirm outside menu); languages row → Settings **Translate**; Soniox context `Select` (hidden if engine ≠ Soniox; disabled while pipeline active).
- No secondary page title — column identity = position + mic/speaker + `aria-label` (no visible “You” / “Meeting” on Live toolbar).
- **Notes session** (`sessionMode === notes`): one unified `paneSurface` wrapping sticky `NotesPipelineToolbar` (mic | Direct\|Notes | speaker) + dual You\|Meeting panes (`variant=notes`, single text). Notes path uses `NotesPathIcon` + `--pipeline-notes-*` amber wash (not Translate cyan). Active capture = meeting timer (no red pulse). Overlay Notes path = icon-only. Interpreters keep per-column Direct\|Translate toolbars.
---

## Column headers

| Column | Identity | Toolbar |
|--------|----------|---------|
| Left | You (aria only) | Single row: Direct/Translate · status + mic flush right |
| Right | Meeting (aria only) | Single row: Direct/Translate · status + speaker flush right |

Implementation: `ColumnPipelineToolbar.tsx`

- One row: `@container flex` — mode cluster | `ml-auto` status icon (`size-6`) + mute (`size-8`) flush right
- Status: **icon-only** (`ColumnHeaderStatus`) beside mute — warnings / reconnect / idle faults only. **No** starting/stopping spinner or custom-active chip (those already live in the Direct/Translate button group). Light border; mute stays the solid control
- **No visible column title** on Live — `title` prop only for `aria-label` / status tips / toasts
- Mode cluster: `inline-flex w-max max-w-[17rem]` path track (not full-bleed); elevation via `--pipeline-path-*` — see Pipeline mode control below
- Meeting Detail History uses `ReadonlyTranscriptTimeline` (single column) — not Live column chrome

---

## Pipeline mode control

- Compact floating path toggle (`h-8`, `text-xs`): **Direct** (`Zap` + label) | **Translate** (`Languages` + label)
- Track + pill: recessed track + **domain-wash active pill** (Direct green / Translate cyan) + strong elevation shadow
- Idle: muted text; hover = text brighten only (no white floating pill competing with active)
- Active: domain wash + hover-level shadow by default
- Icons shared via `pipelineModeIcons.ts` (Live + Overlay)
- Translate segment always shows path icon + label + output mode icon + chevron menu (compact `h-8`; no divider border before chevron). Cyan wash only when Translate is active; idle keeps muted segment chrome with chevron still available
- Picking an output mode from idle also starts Translate (one gesture); while Translate is active, mode changes hot-switch without leaving the path
- Narrow column (`@container` &lt; 260px): icon-only labels; hide output mode icon, keep chevron
- Extra detail via `AppTooltip` (setup / disabled reasons)

**You** output menu (`pipelineLabels.ts`):

- Translated voice
- Custom voice (disabled until custom voice setup in Settings → Voice)
- My voice (raw)
- Captions only

Hot-switch **Translated voice ↔ Custom** while outbound Translate is active is supported (brief silence OK).

**Meeting** output menu: Translated audio · Custom voice · Meeting (raw) · Captions only.

Do not add a third primary mode button on live view.

---

## Status & banners

| Element | Component | When |
|---------|-----------|------|
| Global error | `PipelineErrorBanner` → `LiveNotice.Strip` | Device missing → warning + **Use default** / **Fix**. Other failures → destructive |
| Setup incomplete | `SetupBanner` → `LiveNotice.Strip` | Warning + **Settings** |
| Column connection | `ConnectionBanner` → `LiveNotice.Rail` | “Reconnecting translation” + `n/5`; full copy in tooltip |
| Audio device | `AudioDeviceBanner` → `LiveNotice.Rail` | “Reconnecting audio” + `n/5` (or “Audio restored”). After **5** failed audio reconnects → path **lost**; if a meeting is **live**, engine auto-ends it (persist + Direct), same as user End meeting |
| Session drift | `SessionDriftHint` → `LiveNotice.Rail` | Neutral “Long session” + dismiss |
| Column idle | `ColumnHeaderStatus` | Icon-only `size-6`. `ready`: hidden. When `SetupBanner` visible, hide `setup` / `api-key`. Keep icons for `audio-lost` / `error` / reconnect / checking — label in tooltip only |
| Column active | `ColumnHeaderStatus` | Reconnect / custom-unavailable only. Starting/stopping + custom-active paint in the path button group — do not duplicate. **No** per-column WS elapsed timer |
| Meeting elapsed | `SessionHeaderHub` + `useMeetingElapsed` | Live meeting only; left of language flags |
| Column speech live | `LiveTranscriptColumn` | Live row `border-l-accent` while `buildLiveDisplayState.live` is non-null |

Strip vs rail: same module (`LiveNotice`) — strip is rounded under chrome; rail is full-bleed `border-b` in the column.

Reconnect toasts: `useToast()` in `TranslateView`.

---

## Empty states

| Region | Pattern |
|--------|---------|
| Empty + ready / checking | Tier B centered muted placeholder |
| Empty + `setup` / `api-key` | Tier A `EmptyState` + Settings CTA (`onOpenSettings`) |
| Empty + `error` / `audio-lost` | Tier A `EmptyState` (title/description from badge; no fake CTA) |

Copy (Tier B):

- You: *"Your speech and translation appear here."*
- Meeting: *"Meeting speech and translation appear here."*

---

## Buttons on this screen

| Control | Implementation |
|---------|----------------|
| History | `HistoryButton` — ghost icon, `aria-expanded` |
| Session hub | `SessionHeaderHub` — one-line chip + popover (`headerChrome.ts`); meeting / languages / Soniox context |
| Settings | Ghost icon + warning dot when setup incomplete |
| Setup CTA | `Button variant="secondary" size="sm"` |
| Mic / speaker | `MicMuteButton` / `SpeakerMuteButton` — `size-8 rounded-md` (header chrome parity) |
| Column status | `ColumnHeaderStatus` — icon-only `size-6` (secondary to mute) |
| Pipeline | Icon segment + `ButtonGroup` (Translate always split + mode chevron) |

---

## Interaction

- Resizable columns: `react-resizable-panels` in `TranscriptSplit`
- Segment layout: Settings → App → **Segment layout** — `sideBySide` or `stacked` (`transcriptLayoutStyles.ts`)
- Segment rows: `rounded-lg` inside list pad `px-2` (`transcriptListPadClass`) so live accent border and history shells share the same inset from the pane edge
- Live/focused = `border-l-accent` (+ `bg-secondary/60` when focused — e.g. detail playback seek). Body text: `text-foreground` / source `text-muted-foreground` — never `text-primary`
- Column shell: elevation only; live accent is on the current transcript row
- Mute: immediate; disabled when column audio inactive
- **Full history:** scroll up for committed history (virtualized); **Jump to live** FAB when unpinned; load-earlier spinner near top (*Loading earlier transcript…*)

---

## Accessibility

- Pipeline dropdown: Radix `DropdownMenu`
- Active status: timer/label + color (not color alone)
- Mute: `aria-label`, `aria-pressed`

---

## Live transcript architecture

Read before changing virtualization / interim handoff:

| Topic | Rule |
|-------|------|
| Live tail | **Inside** `@tanstack/react-virtual` as last `displayRows` item — do not split as DOM sibling |
| Stable key | `seq-{direction}-{sequence}` through commit |
| Build | `buildLiveDisplayState(committed, interim)` — one pass per render |
| Context | Per-direction slice — outbound stream must not re-render Meeting column |
| Scroll pinned | `rowCount` → `scrollToIndex`; live text only → scroll to bottom |
| Citation focus | `releasePin` then `scrollToIndex`; state-driven `focused` (not DOM class flash) |
| Row memo | `CommittedTranscriptRowCell` + `LiveTailRowCell` |

**Files:** `LiveTranscriptColumn.tsx`, transcript context under `src/features/pipeline/context/`, `transcriptView.ts`.

## Agent notes

- Use shadcn + `useToast()` only
- Session hub: one-line chip (flags + `ArrowLeftRight`, no unicode `⇄`); full names/title/context in tooltip + popover
- Path / status icons: Lucide via `pipelineModeIcons.ts` + `ColumnHeaderStatus`
- Header hub shares `src/shared/lib/headerChrome.ts` (`h-8`, `rounded-md`, truncate)
- Split resize: mid-gutter `GripVertical` via `TranscriptResizeHandle` (no full-height line)
- Live toolbar is independent of History timeline chrome
