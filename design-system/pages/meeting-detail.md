# Page override: Meeting Detail

> Overrides `design-system/MASTER.md` where noted below.

**State:** `appView === "meeting-detail"`
**Root:** `src/features/meeting/detail/components/MeetingDetailView.tsx`
**Related:** `MeetingDetailLayout.tsx`, `MeetingAudioPlayer.tsx`, `SummaryPanel.tsx`, `SummaryGenerateChip.tsx`, `ReadonlyTranscriptTimeline.tsx`, `MeetingDetailActions.tsx`

---

## Layout

```
┌─────────────────────────────────────────────────┐
│ [← Live]  │ title + meta (chip)         │ [⚙]│  ← click chip → actions
├──────────────────┬──┬───────────────────────────┤
│ Summary (~38%)   │⋮│ Transcript (~62%)          │  ← resizable PanelGroup
│                  │ │                    [🔍]   │  ← floating search (right)
│ summary body [✎] │ │ ┌───────┬↑↓×┐             │  ← find bar when open
│                  │ │         ┌───────────────┐  │
│ Artifacts ▾      │ │         │ −5 ▶ … 1× [≡] │  │  ← player + source menu
│       [Sparkles]   │ │         └───────────────┘  │  ← AI dock (Generate only)
└──────────────────┴──┴───────────────────────────┘
```

- **No tabs** — Summary and Transcript are always visible side-by-side via `react-resizable-panels`.
- Default sizes: Summary **38%** | Transcript **62%**; `minSize={28}` each; persist with `autoSaveId="meeting-detail-split-v1"`.
- Pane insets match Live: shell `p-3` + content `p-0` — **no** extra `px-1` / `pb-2` on the detail PanelGroup
- Mid-gutter: shared `TranscriptResizeHandle` (same Live grip pattern).
- Meeting title lives in the **app header center** (`MeetingDetailHeaderTitle`) — not a second title band in `app-main`.
- Session hub is **hidden** on this view; **no pane toolbar rails** (Summary / Transcript are headerless)
- **MeetingAudioPlayer** floats over the Transcript pane (bottom inset) — not a docked footer: −5 / play / +5 · scrubber · speed cycle (`1×` → `1.25×` → `1.5×` → `2×`) · **source filter** icon (`ListFilter` → All / You / Meeting)
- Find-in-transcript: floating **Search** chip (top-right, `right-4` scrollbar gutter) opens a **compact floating bar**: input · `N/M` | ↑ ↓ × — Esc / × / toggle off closes
- Duration lives only on the **app-header chip meta** (not repeated on the transcript pane)

---

## Title (app header)

- Center: **capped chip** `max-w-[min(24rem,50vw)]` (not a full-bleed banner): **title** (truncate, semibold) + **meta** line (`Jul 26 · 3 min · VI → EN`) + chevron when menuable
- Meta via `formatMeetingDetailChipMeta` — date · duration (if ended) · language pair (notes: single lang)
- Language pair and duration appear on the chip face + tooltip (not on the transcript pane)
- Click chip opens meeting menu (Rename / Move / Delete) — **no** separate ⋯ button (Live hub pattern).
- Rename uses auto-growing textarea (`min-h-9`, `max-h-32` / 128px) — Enter saves, Shift+Enter newline, Esc cancels
- Omit **Ended** and **Notes** badge from chrome (date/duration/langs on meta line; clock start time remains in tooltip).
- Header grid: `auto | minmax(0,1fr) | auto`; chip is centered and width-capped so long AI titles truncate instead of stretching edge-to-edge.
- Chip tooltip (stacked lines): **name** · **date · time** · **languages** — no Ended
- Delete: `ConfirmDialog` — never immediate.
- Live meetings: chip is display-only + **Live** badge (rename/end via Live hub); meta omits duration.

---

## Summary pane

- Elevated `paneSurface`; **no pane toolbar rail** — content-first body
- **AI dock** (bottom-right overlay): **Sparkles** FAB opens a narrow upward **Popover** (`w-52`, no header) with shared `Select`s for template + language + Generate/Regenerate; click-outside dismisses. Spinner + `aria-busy` on the FAB while generating
- Edit / Save / Cancel live **on the summary document**, not in the dock: review mode shows a **sticky** Edit chip (top-right, no full-width wash) matching transcript Search (`outline` + `paneToolbarActionChip`); hidden while editing; edit mode uses a sticky footer (hint + Save / Cancel) inside the summary detail shell (editor scrolls)
- TipTap summary body (**SummaryKit**): vocabulary = generate seed (`h3` section · accent-hairline bullets · bold · citation chips). Node styles on kit `HTMLAttributes` (`summaryKit.ts`), not open StarterKit / not `@tailwindcss/typography`. Review = flush; Edit = light `bg-secondary/25` + focus ring
- Edit affordances (no format strip): type `/` → Section | Bullet list; `@` or transcript click → citation; text selection → BubbleMenu **Bold** only. Suggestion popovers: active = accent rail + `bg-accent/12`; idle hover = `bg-muted` (never same token). Hint: *Type / for section or list · click a transcript line or type @ to cite.*
- Summary scroll body has **no** FAB padding — the Artifacts panel occupies the pane bottom, so the safe zone lives there instead
- The **ArtifactsPanel** scroll list uses `pb-14` so the last row's overlay `⋯` menu clears the AI dock
- Artifacts header is **disclosure-first** (collapse chevron on the left): when collapsed the header sits at the pane's bottom edge where the dock covers the right side — never put interactive chrome on that header's right end
- Artifact rows are **checklist rows**: one leading toggle carries both state and confirm action (empty circle → accent-filled `CheckCircle2`, click again to undo back to `proposed`; `aria-pressed`). Confirmed state reads from the accent border + filled check — **no "Confirmed" text badge** (state is chrome, not content). Owner/due meta and citations indent to the text edge (`pl-[1.625rem]`)
- Extraction has **one entry point** — the AI-dock Generate flow (artifacts ride on summary generation; the panel refetches via `summaryGeneratedAtMs`). Empty state hints point at **Summary options** below (no emoji in product copy)
- **Curation:** rows are user-editable via a trailing overlay `⋯` (`MoreHorizontal`) that **does not reserve idle width** — text stays full-bleed. Hover/focus (or always on coarse/no-hover pointers) reveals the trigger; a **compact** menu (`text-xs`, tight padding) holds Edit / Dismiss / Delete (second arm label: Confirm). Edit/add use the same inline form (required `Textarea` text + optional `Input` owner/due), `Enter` saves / `Esc` cancels. Destructive delete is **two-click confirm in the menu**, no dialog. A ghost dashed `+ Add …` row closes each section; manual rows carry an "Added manually" tooltip. **Mentioned** chips use the same true overlay `⋯` (Rename / Delete) — no idle padding inside the badge; trailing `+ Add` chip stays. Rows stay checklist-first; curation never adds text badges (state is chrome, not content)
- Errors stay inline in the panel (`InlineError`)
- Empty (no inbound transcript): `EmptyState` with `FileText` — no Generate button
- Citation chips: compact `m:ss·You|Meeting` (no brackets); insert via `@` suggestion or transcript click; outbound gets subtle accent border

---

## Transcript pane

- `ReadonlyTranscriptTimeline` — **single chronological column** (You + Meeting interleaved by `startedAtMs` / `sequence`)
- **Not** a Live-style You | Meeting resizable split — History optimizes for turn-taking readability; Live keeps the You|Meeting split
- Force **stacked** Original / Translation inside each segment
- Surface hierarchy: elevated `paneSurface` / `--pane-elevated` · body `bg-card` — **no toolbar rail**
- Floating **Search** chip (top-right with scrollbar gutter `right-4`, matching top inset; hidden with zero segments **or while the find bar is open**) opens the find bar — duration is **not** on this pane (app-header chip only)
- Find bar (open): compact elevated pill top-right (same gutter) — **input** · **`N/M`** | **↑ ↓ ×**; Enter/↓ next · Shift+Enter/↑ prev · Esc/× close
- Notes mode: single language flag (like Live Notes hub)
- Each row: inside the segment shell — **one meta row** (`time` + You/Meeting) then stacked transcript text; speaker label sentence-case muted (You keeps light primary); Notes mode same pattern
- Zero segments: `EmptyState` — **Start new meeting** / **Delete meeting**; transcript-dependent features hide instead of disabling: find-in-transcript trigger, player source filter (see "No-transcript gating" below)
- Filtered empty: Tier B centered muted placeholder
- Omit Live-only chrome: Direct/Translate, mute, status/banners, Jump-to-live

---

## Empty states & errors

| Case | Pattern |
|------|---------|
| Zero segments | `EmptyState` + primary / destructive actions + no-transcript gating (below) |
| Summary, no transcript | `EmptyState` — no generate CTA |
| Summary API failure | Inline in Summary pane |
| Page load failure | In-view error text — not app crash |
| No meeting audio | Floating player hint — enable Save meeting audio in Settings |

---

## No-transcript gating (`segments.length === 0`)

Transcript-only features **hide** (not disable) — a muted unusable control reads as broken chrome:

- **Find in transcript** — floating Search trigger unmounted
- **Artifacts panel** — empty copy switches to "No transcript to analyze…" (live meetings keep the "after the meeting ends" copy); no in-panel CTA exists to hide since extraction rides on the Generate dock flow
- **Player source filter** (All / You / Meeting) — hidden in all player states (loading / no-audio / active); the player itself stays — audio can exist without segments

Kept visible: player controls, Summary pane (and Summary options FAB when transcript/live allows), Start new meeting / Delete meeting actions.

---

## Audio player

- **Floating overlay** on the Transcript pane (`absolute` bottom inset) — not a docked `border-t` footer that consumes layout height
- Shell: `rounded-lg border bg-secondary` + `--pane-elevated` shadow + light ring — must read above transcript `bg-card` (do not reuse flat `bg-card`)
- Transcript list reserves bottom space (`bottomInsetClassName` / `pb-20`) so the last rows stay readable under the player
- Controls: **−5** · play/pause · **+5** · scrubber · time labels · **speed** button (`1×` → `1.25×` → `1.5×` → `2×`, one click cycles) · **source** icon menu (**All** / **You** / **Meeting**) — also filters the transcript list
- Seek / play / skip advances transcript highlight (±500 ms of `startedAtMs`)
- Click timeline segment → seek audio to that `startedAtMs` and play
- Empty / loading: same floating shell; the source filter shows only when the meeting has a transcript (no-transcript gating)

---

## Buttons

| Control | Implementation |
|---------|----------------|
| Back to Live | `Button variant="secondary" size="sm"` + `ArrowLeft` |
| Summary Generate | AI-dock **Sparkles** FAB → upward Generate sheet (defaults-first template/language) |
| Transcript search | floating **Search** chip (hover tooltip; elevated) → floating find bar |
| Player skip | `−5` / `+5` outline `xs` |
| Player speed | single outline button cycling `1×` · `1.25×` · `1.5×` · `2×` |
| Player source | `ListFilter` icon → dropdown All / You / Meeting (`AppTooltip` on trigger + each option) |
| Delete | `AppButton destructive` → confirm |

---

## Loading

`MeetingDetailSkeleton` mirrors the detail chrome (body only — header stays in `AppShell`):

- Split **~38% / grip / ~62%** with elevated `paneSurface`
- Summary: body line stubs + Edit stub; AI dock stub (Sparkles circle) bottom-right
- Transcript: floating **Search** chip stub top-right (`right-4`); row stubs; floating **player** bar bottom

---

## Live meeting in detail

- Poll refresh while `meeting.status === "live"`
- Header back returns to live view

## Agent notes

- Summary anchors scroll a **single** virtualized list (no per-direction pane); if the active filter hides the segment, reset filter to **All** then scroll
- Do not reintroduce Summary | Transcript tabs on this screen
- Citation chips use `AnchorTimeBadge` — see `design-system/MASTER.md` and this file § Summary pane
- Related: `MeetingAudioPlayer.tsx` — Rust decode Ogg Opus windows via `decode_meeting_audio_window`; do not rely on WebView Opus decode
