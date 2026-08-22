# Page override: Overlay

> Overrides `design-system/MASTER.md` where noted.

**State:** Overlay `WebviewWindow` visible (`label: "overlay"`) or Settings App-tab preview
**Root FE:** `src/features/overlay/`

---

## Window chrome

```
┌──────────────────────────────────────────────────┐
│ ⠿ Meeting title  [click-through] [opacity] [−]   │  ← drag; quick toggles; hide
│──────────────────────────────────────────────────│
│ [mic] [⚡|🌐·👤▾] [chip] │ [spk] [⚡|🌐▾] [chip] │  ← mute · path · status
│ translation…             │ translation…          │
│ (source muted)           │ (source muted)        │
│                                    [⌟]           │  ← resize (click-through Off)
└──────────────────────────────────────────────────┘
```

| Spec | Value |
|------|-------|
| Default size | 420×280 logical (`overlay.width` / `overlay.height`) |
| Size range | 320–900 × 200–700; persisted via `overlay_set_size` |
| Decorations | None (`resizable(false)` — UI handle only) |
| Always on top | Yes |
| Skip taskbar | Yes |
| Background | `color-mix` alpha on `--card` via opacity setting — text stays full contrast |
| Focus | Do not steal on text update |

Position presets reset `offsetX`/`offsetY` only — **saved width/height are kept**.

---

## Content

- Tail of recent segments (~40) + live interim
- Two direction columns (outbound | inbound); no You/Meeting labels — 1 column when a direction is filtered off
- Header title = active meeting name (`meetingChipDisplayTitle`); fallback `Meeting` / `Preview`
- Within a column, layout follows app `transcriptLayout` (stacked = source over translation; sideBySide = source \| translation)
- Live / interim rows use accent border only; body text stays `text-foreground` (source muted) — same as Live
- Stick-to-bottom scroll (`useStickToBottomScroll`); compact Jump FAB when unpinned. Hook avoids rAF (throttled when overlay is unfocused) and re-sticks via ResizeObserver + window focus.
- Empty: short muted placeholder (“Waiting for speech…”)

---

## Live controls (overlay window only)

Compact per-column strip (single row): **mute → path/mode segment → status chip**.

| Control | Behavior |
|---------|----------|
| Mic / Speaker | Same invokes as Live; disabled when `!muteEnabled` |
| Path segment | Borderless floating track (`bg-secondary/90`, no border). Active pill: `bg-card` / `dark:bg-hover-surface` + `shadow-sm`. Icons stay full contrast. `Zap` = Direct (`--pipeline-direct-text` when active), `Languages` = Translate (pipeline translate text + mode icon + chevron). Mode menu keeps solid `bg-popover`. Hover via `AppTooltip` |
| Output menu | Custom fixed list (no Radix portal) — works with click-through + hold modifier; holds pointer interactive while open |
| Status chip | Icon-only states; full text via `AppTooltip` |
| Header / path / mute | Path/mute: `AppTooltip`. Header chrome: no tooltips (avoids popups while dragging) |
| Header opacity | Cycles 0.70 → 0.92 → 1.00 via `overlay_patch_settings` |
| Click-through badge | Read-only: `Click-through · hold Ctrl` (Windows) / `hold Cmd` (macOS) when ON |
| Resize handle | Bottom-right SE grip; **only when click-through Off** (hidden when On, even with hold modifier). Live `setSize` on drag; persist on pointerup |

Settings App-tab **inline** preview (`compact`) reuses the same `OverlayTranscriptView` + disabled control chrome (no invokes). Height ~200px — **no resize handle**.

**Open window** calls `overlay_preview` — same control chrome as Tray, seeded with the same mock transcript as the inline card. Bootstrap via `overlay_is_preview_mode` if the webview mounts after the event. Click-through follows Settings (hold Ctrl / Cmd to interact when ON) — same as the live overlay.

---

## Settings (App tab)

Section **Overlay** between Transcript display and Preferences — see [settings.md](settings.md).

Instant-save via shadcn `Checkbox` / compact `ButtonGroup` (short position labels) / `SettingInfoHint` / `SettingsGroup`.  
Inline preview (~200px, same chrome as live window, controls disabled) + **Open window** beside the Preview label.

### Transcript overlay (Off / On)

Single segmented control **Off / On** — transcript-only overlay window.

| Mode | Saved as | Behavior |
|---|---|---|
| Off | `enabled=false` | No window; dependent controls dim. Toggle via Settings, Tray, or `Ctrl/Cmd+Shift+O` |
| On | `enabled=true` | Always-on-top transcript window for live translations |

Tray "Show transcript overlay" and the toggle hotkey flip `enabled` only.

### Hide from screen share

- Default ON
- Hint must be platform-honest: Windows = omitted from share; macOS 15+ = best-effort

---

## Interaction

| Mode | Behavior |
|------|----------|
| Click-through OFF (default) | Overlay receives all pointer events — drag, resize, hide, mute, D/T |
| Click-through ON | Pass-through by default. **Hold Ctrl** (Windows) / **Cmd** (macOS) to interact; release to pass through again. Mode menu holds interactive until closed. Mode toggle: Settings, Tray, or `Ctrl/Cmd+Shift+T` |

Rust owns ignore state (`overlay/clickthrough.rs`): polls global Ctrl (Windows) / Cmd (macOS) ~80ms while click-through is desired; applies `set_ignore_cursor_events` only on change (Windows Win32 / macOS AppKit fast path). Emits `overlay-pass-through` when ignore turns on so FE can blur stuck hover chrome (no `pointerleave` while pass-through). Preview seeds mock text only — does not bypass click-through. `apply_settings` does not re-`show` an already-visible overlay (avoids one-shot ignore bugs).

**Perf:** Live interim rows use stable ids and update in place; transcript UI commits are throttled (~80 ms); tail is capped at **20 rows per direction** (preserves chronological order). Overlay scroll observers skip `characterData`. No tree-wide `ClockProvider` — session timer ticks locally only while Translate is active. Click-through modifier poll ~80 ms. `OverlayColumnControls` is memoized so transcript text updates do not rebuild chrome. Overlay transcript clears when the active meeting ends or changes (same as Live `clearTranscripts`).

Header uses `data-tauri-drag-region=""` plus `startDragging()` fallback. Hide / opacity / resize set `data-tauri-drag-region="false"`.  
Move/resize persist: debounced `onMoved` → `overlay_set_offset`; SE handle → `overlay_set_size` (rebases offset so size-dependent anchors do not jump the window).

---

## Agent notes

- Reuse `OverlayTranscriptView` for preview and window — do not fork styles
- Do not embed full `ColumnPipelineToolbar`; use `OverlayColumnControls`
- No cards in the floating chrome beyond the panel itself
- Lucide only; no emoji
- Do not apply CSS `opacity` on the whole panel (fades text)
- Do not enable OS `resizable(true)`
