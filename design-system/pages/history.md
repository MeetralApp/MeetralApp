# Page override: History (Library Drawer)

> Overrides `design-system/MASTER.md` where noted below.

**State:** `libraryOpen === true` (Sheet overlay)
**Root:** `src/features/meeting/library/components/LibraryDrawer.tsx`
**Related:** `LibraryTree.tsx`, `LibrarySearch.tsx`, `LibrarySortableFolderList.tsx`, `LibraryMeetingRow.tsx`, `LibraryFolderBlock.tsx`

---

## Layout

### Browse mode (search closed or empty query)

```
┌──────────────────────────┐
│ History              [X] │
├──────────────────────────┤
│ CURRENT SESSION          │  ← only when a live meeting exists
│   · Live meeting row     │
├──────────────────────────┤
│ RECENT                   │
│   · Meeting row(s)       │     max 3 ended meetings
├──────────────────────────┤
│ FOLDERS            [+]   │
│   ⠿ ▼ Folder name  (n)   │
│       · Meeting row      │
│   ▼ Uncategorized  (n)   │
├──────────────────────────┤
│ ┌ Search transcripts… ×┐ │  ← floating field when search toggle open
│ [🔍] [+ New meeting] │
└──────────────────────────┘
```

### Search mode (toggle open + non-empty query)

```
┌──────────────────────────┐
│ History              [X] │
├──────────────────────────┤
│ RESULTS                  │
│   Meeting title          │
│   Jul 26 · VI → EN       │
│   “…the ▍advocate▍…”    │  ← highlight query terms
│   1:05                   │  ← segment time (meeting-relative)
│   ···                    │
├──────────────────────────┤
│ ┌ advocate            ×┐ │  ← floating above footer
│ [🔍] [+ New meeting] │
└──────────────────────────┘
```

| Spec | Value |
|------|-------|
| Component | shadcn `Sheet` side `"left"` |
| Width | `w-[320px]` (`max-w-[92vw]` on small) |
| Header | `border-b px-3 py-2` |
| Body | `px-3 pt-1.5`, flex column, overflow hidden |
| Footer | `border-t` — pinned; **Search toggle** + **New meeting** |
| Close | Ghost icon `X` |
| ARIA | `aria-label="Meeting history"` |

**Mode rule:** When the search query is non-empty (after trim), **hide** Current / Recent / Folders and show a full-height **Results** list. Do not nest a small results box above the browse tree.

---

## Search (`LibrarySearch`)

| Spec | Value |
|------|-------|
| Trigger | **Search** icon toggle beside **New meeting** — hover tooltip; `paneToolbarActionChip`; pressed adds ring |
| Field | Floating shell **above** the footer when toggle open: input · close `X`; `rounded-lg` + elevated `bg-secondary` |
| Focus | Auto-focus input on open; **Escape** closes search |
| API | `searchMeetings` → `MeetingSearchHit` (1 best hit per meeting) |
| Debounce | 280ms |
| Limit | 20 |
| Status line | `{n} match(es) in transcripts` · `Searching…` · error (in results pane) |
| Click hit | Close drawer → Meeting Detail + scroll to `bestSegmentId` |

### Result row hierarchy

| Layer | Content |
|-------|---------|
| Primary | Meeting display title (`meetingChipDisplayTitle` when record known; else `hit.title`) |
| Meta | Date · language pair (from meetings list enrichment; omit unknown parts) |
| Snippet | `line-clamp-2`; **highlight** query tokens (`bg-accent/40 text-foreground`) |
| Anchor | Segment timestamp via `formatSegmentTimestamp(hit.startedAtMs)` when present |

### Search states

| State | UI |
|-------|-----|
| Idle (toggle closed) | Browse tree; footer `[🔍] [New meeting]` |
| Open (empty query) | Floating input above footer; browse tree still visible |
| Searching (no prior hits) | Tier B *"Searching…"* in results pane |
| Hits | `SectionHeading` **Results** + scrollable list (`flex-1 min-h-0`) |
| Empty | Tier B *"No matches for '{query}'"* |
| Error | `text-destructive` inline |

**Out of scope (phase 1):** date-range filters, Meetings vs Segments tabs, command palette.

---

## Sections (browse mode)

### Current session

- Shown only when there is an active live meeting
- Row returns user to live view; meta may read *Current session · Return to live*

### Recent

- Label: `SectionHeading` — **Recent**
- Up to **3** most recent **ended** meetings (`RECENT_LIMIT`)
- Empty: *"No recent meetings"* (Tier B)

### Folders

- `LibrarySortableFolderList` — drag reorder via `@dnd-kit`
- Drag handle: `GripVertical`, `aria-label="Reorder folder"`
- `+` creates folder; expand/collapse, rename/delete, Uncategorized
- Selected folder drives **New meeting** target

### Meeting rows

- Browse-only — click opens Meeting Detail (ended) or live (active)
- No per-row overflow menu (actions on Meeting Detail)
- Title / meta via meeting display helpers
- **Content signals** (Notes mode, has summary) live on the meta line — never as
  title-row badges; title row chrome = live status only (live dot + LIVE badge).
  Meta = `{date ·} {lang pair} {· Notes} {· Summary}` with `·` separators.

---

## Footer CTA

```tsx
<Button variant="secondary" size="sm" className="w-full">
  <Plus size={16} /> New meeting
</Button>
```

- Active live meeting → `InlineConfirm` before ending and starting new
- Target folder = selected folder or Uncategorized
- Success: close drawer + toast via parent

---

## Empty states

| Case | Pattern |
|------|---------|
| No recent | Tier B muted |
| Empty folder | Tier B *"No meetings"* |
| Search empty | Tier B *"No matches for …"* |
| Loading / error | Muted or `text-destructive` status |

---

## Navigation

| Action | Result |
|--------|--------|
| Ended meeting | Close → Meeting Detail |
| Live meeting | Close → Live |
| Search hit | Close → Meeting Detail + scroll to segment |
| Escape / overlay / X | Close drawer |

---

## Typography & density

- Denser than Settings — scan-heavy list
- Row title `text-sm`; meta `text-xs text-muted-foreground`
- Search snippet `text-[11px]` / `text-xs`
- Section labels: `SectionHeading` only

## Accessibility

- Sheet focus trap (Radix)
- Meeting row / search hit: single button target
- Search input: `aria-label="Search meeting transcripts"`
- Clear: `aria-label="Clear search"`
- `HistoryButton`: `aria-expanded`, `aria-haspopup="dialog"`
- Tooltips: `AppTooltip` only — no native `title` on chrome (meeting rows, folders, icon actions, New meeting)

## Agent notes

- Preserve `@dnd-kit` folder reorder patterns
- Keep **Current session** as its own section above Recent
- Prefer `AppTooltip` for truncated titles / icon buttons (MASTER § tooltip)
- Search mode **replaces** browse tree — never show both
- `MeetingSearchHit.startedAtMs` is **segment** meeting-relative ms (not meeting wall clock)
)
