# Page override: Settings

> Overrides `design-system/MASTER.md` where noted below.

**State:** `drawerOpen === true`
**Root:** `src/features/config/components/SettingsDrawer.tsx`
**Tabs:** `settingsDrawerTabs.tsx` · Focus: `lib/settingsFocus.ts`

---

## Layout (5 tabs)

**Order (left → right):** Translate → Voice → Audio → Intelligence → App

```
┌────────────────────────────────────┐
│ Settings            Unsaved?   [X] │  ← optional Unsaved when draft dirty
├────────────────────────────────────┤
│ Translate│Voice│Audio│Intelligence│App│  ← TabsList line + warn dots
├────────────────────────────────────┤
│ (scrollable tab content)           │
│   … section footers Cancel/Save …  │
└────────────────────────────────────┘
```

| Spec | Value |
|------|-------|
| Component | shadcn `Sheet` side `"right"` |
| Width | `w-[440px] max-w-[92vw]` |
| Header | `border-b px-3 py-2` — `h2` sheet title + optional “Unsaved” micro-status + ghost `X` |
| Tabs | `Tabs` + `TabsList variant="line"` |
| Body | `px-[18px] py-2 pb-6`, `overflow-y-auto` per tab |
| Tab warn | 6px `bg-warning` dot when setup/dirty |

Closing with dirty drafts → `ConfirmDialog` discard; content remounts clean on next open.

---

## In-tab anatomy (all tabs)

One tab = vertical list of **L1 sections**. No nested cards. No free-floating toolbars outside a section.

```
Tab panel
└── SettingsSection (L1) × N          ← soft `border-b border-border/40`, py-4
    ├── Header: title | hint | headerEnd | status?
    ├── Body: gap-4
    │   ├── SettingsGroup (L2)? × M   ← only when ≥2 peer groups; gap-only
    │   └── SettingsField (L3) × K
    └── Draft footer? (Cancel + Save when section dirty)
```

### Four primitives

| Primitive | When | Do not |
|-----------|------|--------|
| `SettingsSection` (L1) | Functional cluster in a tab | Fake L1 via bordered cards. Section rule: soft `border-b border-border/40` (not full-opacity `border-border`) |
| `SettingsGroup` (L2) | Peer groups inside one section | Add when section has a single flat stack — omit L2 |
| `SettingsField` (L3) | Every control row | Ad-hoc label/layout classes |
| Section draft footer | Dirty draft sections | Drawer-wide Save-all |

**Collapsible Advanced:** `SettingsGroup` + `Collapsible`, trigger = `SectionHeading` (L2). Never `rounded-md border` + L1-weight title.

### SettingsField row

```
[L3 label + ?hint]     [secondary actions]
[control full width]
[L4 description optional]
```

Field stack `gap-3`; section body `gap-4`.

**Header row height:** L1 / L2 / L3 title rows use `min-h-7` + `items-center` so title-only and title+actions (API key / Refresh / Preview) share the same vertical band — gap to the control stays consistent.

### Action slots

| Slot | Allowed |
|------|---------|
| **`headerEnd` (L1/L2)** | `ApiKeyChip` **or** one catalog **Refresh** — icon-only via `SettingsIconButton` + tooltip |
| **Field trailing (L3)** | Refresh / Preview — same icon-only pattern (`Play` / `RefreshCw`) |
| **Toolbar row** | Audio only: Refresh + Auto-fill — **keep text labels** (paired with long Auto-fill) |

Overlay open action: label **Open overlay** / **Open** — do not reuse voice “Preview”.

### API key pattern

| State | UI |
|-------|-----|
| Missing | Full `SecretApiKeyField` under owner (Engine / Intelligence Provider / ElevenLabs / Fish Audio) — no duplicate L1 “API key” section once configured path exists |
| Ready | Icon-only `ApiKeyChip` (Key + status tone) on **owner header**; tooltip `API key · {status}`; expand → `border-t` + field with `showLabel={false}` |
| Dirty | Auto-open panel; status “Not saved”; must feed drawer `hasUnsavedChanges` (including Intelligence) |

Owners: Engine field (Translate cluster), Intelligence Provider block header, ElevenLabs L2, Fish Audio L2.

### Typography

| Level | Role | Style |
|-------|------|-------|
| L0 | Sheet title | `text-base font-semibold text-foreground` |
| L1 | Section | `text-sm font-semibold tracking-tight text-foreground` |
| L2 | Group | `SectionHeading` (`text-xs` uppercase muted) |
| L3 | Field | `settingsFieldLabelClass` |
| L4 | Description | `text-xs text-muted-foreground` |

**IA rules:** never repeat the tab name as L1; omit L2 for a single control; peer provider blocks (ElevenLabs / Soniox TTS) both L2; ≤1 intro line under L1 (long tips → `SettingInfoHint`).

---

## Tab contents

### Translate

| Order | Section | Component |
|-------|---------|-----------|
| 1 | **Engine + Live model + Languages** | One L1 cluster (no rules between fields): Engine + `ApiKeyChip` when configured; missing key as L3 under Engine; Live model + Refresh; Languages pickers |
| 2 | **Context** | Soniox only — Always-on + Meeting profiles |
| 3 | **Speech detection** | Gemini only — L1 titled section; fields flat or L2 Advanced collapsible (no card-L1) |

### Voice

Two L1 sections (do not wrap in a redundant “Voice” L1). Status badge on **Meeting → You**.

| Section (L1) | Content |
|--------------|---------|
| **Meeting → You** | L3 mode; Custom voice engine (ElevenLabs \| Fish Audio) when custom voice is on; Soniox → L2 Soniox TTS (**shared TTS model on both Engine columns**, same field; per-direction voice + speed + Preview on voice field); Gemini/OpenAI Engine → session note; vendor L2 + Advanced L2 collapsible |
| **You → Meeting** | Same anatomy |

Custom voice API keys once per vendor on the first visible custom voice group that uses that vendor. ElevenLabs Advanced: Speaking style (hidden for Soniox live engine), TTS model (`SettingsField` + Refresh), Stability, Similarity. Fish Audio Advanced: TTS model (`SettingsField` + Refresh, same chrome as ElevenLabs), latency, temperature, speed, top-p. Draft footer when dirty.

### Audio

Section **Routing** → `AudioDeviceSettings`:

- Toolbar: **Refresh** · **Auto-fill VoiceMeeter** / **BlackHole**
- Role labels (short + description):

| Role | Label | Description |
|------|-------|-------------|
| userMic | Your mic | What you speak into |
| teamsMicFeed | To meeting | Sent to the meeting as your mic |
| meetingCapture | From meeting | Captured from the meeting |
| localPlayback | Playback | Where translation plays |

- **Keep Direct audio** checkbox in this section (not App)
- **Save meeting audio** (default Off):
  - When on: folder path + Change / Open / Reset; `Badge` total recorded size (MB/GB) via `SUM(audio_chunk.byte_size)` (size stored at write; no FS walk)
  - When off but recordings exist: same size badge beside the label
- **Hear who is speaking** (Meeting → You):
  - Toggle (default On); when on, shows Quiet volume slider (same show/hide pattern as Save meeting audio)
  - Slider: Quiet volume (0–50%, continuous underlay — no delay-align / no idle duck)
  - Dragging Quiet volume hot-previews on live inbound; **Save** persists (Cancel restores saved level)
  - Hint: headphones; Playback must not feed From meeting
- Hidden roles: dashed note; Setup guide; Cancel + Save when dirty

### Intelligence

Tab id `"intelligence"` (label **Intelligence**). Deep-link aliases `"summaries"` / `"summary"` still resolve here.

Tab owns the name — omit redundant L1 “Intelligence” while only one cluster; four `SettingsGroup`s:

| Group | Content |
|-------|---------|
| **Provider & model** | Summary provider (Gemini \| OpenAI \| custom profile) + API key chip/field + summary model + **AI answer language** select (`""` = Match meeting language). Hint: powers meeting summaries. Keys shared with Translate when same provider. |
| **Custom providers** | OpenAI-compatible LLM profiles (label, base URL, chat model, API key) for summary generation |
| **Meeting context** | Collapsible domain/terminology/background for summaries (draft + Save; separate from Soniox live context) |
| **Features** | **Artifacts** toggle — OFF hides the panel + BE rejects with `feature_disabled:` |

- Keys shared with Translate when same provider (hint)
- Model disabled until key ready (built-in providers)
- Custom profile selected → model row hidden; edit via Custom providers group
- Instant persist for toggles, model, language (toast on save)
- API key remains draft + Save (discard dialog)

### App

| Section | Content |
|---------|---------|
| **Transcript display** | Segment layout (Side by side \| Stacked) |
| **Overlay** | L2 Appearance / Behavior; Open overlay (not “Preview”) |
| **Preferences** | L2 **Theme** + Application (tray); no Keep Direct |
| **About** | `AboutFooter` |

---

## Save policy

| Pattern | Examples |
|---------|----------|
| Instant | Engine, languages, live/summary model, Soniox TTS voice/speed, voice mode, segment layout, overlay, theme |
| Draft + Save | API keys (Translate + Intelligence), Audio devices, Speech detection, Soniox context edit, custom voice Advanced, tray prefs |

All drafts (including Intelligence API key) → drawer discard dialog.

Deep link: `focusKey` + `activeFocus` → `scrollIntoView`.  
`resolveSettingsTab()` maps `SettingsFocus` → tab.

| Tab | Focus aliases |
|-----|---------------|
| Translate | `translate`, `api`, `provider`, `languages`, `sonioxContext` |
| Voice | `voice`, `customVoice` |
| Audio | `audio` |
| Intelligence | `intelligence`, `summaries`, `summary` |
| App | `app`, `overlay` |

### Locks

| Area | Lock when |
|------|-----------|
| Engine / Languages | Any pipeline busy |
| Meeting → You / inbound custom voice knobs | Inbound active/starting |
| You → Meeting / outbound custom voice knobs | Outbound active/starting |
| Shared custom voice API keys (EL / Fish) | Either direction that uses that vendor is active/starting |

---

## Help / a11y

- Hints: `SettingInfoHint`
- Secrets: `SecretApiKeyField`
- Toasts: `useToast()`
- Sheet: `aria-label="Settings"`

## Agent notes

- Keep five-tab IA — Intelligence owns summaries, custom LLM profiles, meeting context, and artifacts
- Always show Meeting → You and You → Meeting as sibling L1 on Voice
- Summary provider is independent of live engine (never Soniox)
- Paths: settings UI under `src/features/config|ai|voice|audio|overlay/`, not `src/components/`
