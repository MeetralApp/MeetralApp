# xAI TTS — chase Soniox translate (not sentence-batch)

**Status:** working proposal (not SSOT).  
**Constraint:** do not wait for a full sentence before TTS — that lags behind Soniox translate.  
**Scope:** Soniox live → Custom xAI only. Gemini / OpenAI fanout and current xAI mapping stay unchanged.

---

## Why not sentence-batch

Waiting for `.?!` before `text.delta`/`text.done` means TTS cannot start until the translator has a whole sentence. The transcript column already shows tokens. That gap is exactly the extra delay to avoid.

xAI’s streaming contract for chase is the opposite of batching:

- `text.delta` as text arrives
- `audio.delta` **may** start once the server has “enough text”
- `text.done` only when **this utterance is finished** (Soniox `turn_complete`)

Closing the utterance early (`text.done` per sentence or per phrase) forces `XaiTurnGate` to queue later tokens until `audio.done` → TTS falls further behind translate.

---

## Isolation (Gemini / ChatGPT untouched)

| Live engine | Fanout | xAI behavior |
|-------------|--------|----------------|
| Gemini / OpenAI | `FanoutKind::ElevenLabsDelivery` | **Unchanged** — current worker + `Flush` → `text.done` |
| Soniox | `FanoutKind::ProviderTts` | **Chase mode** in `providers/xai` only |

Pass a flag from factory/voice runtime using **`uses_separate_tts`** (capability), not `match AiProvider` in pipeline/mux/engine. Do not change Soniox fanout in a way that affects EL/Fish on Soniox. Do not change `elevenlabs/delivery/**`.

---

## Chase mode (one open utterance per Soniox turn)

Keep the utterance **open** until `turn_complete`. Stream translate suffixes immediately. Never `text.done` mid-turn just to “flush” — xAI has no keep-open flush; `text.done` **ends** the utterance and stalls the next tokens.

```
Soniox prefix token  →  text.delta (now)
        … more tokens →  text.delta (now)
turn_complete        →  text.done
audio.done           →  next turn (already queued in XaiTurnGate)
```

This matches REST: *“audio generation begins as soon as enough text is buffered.”* and product copy: play audio before the full text is available.

### A. Start audio sooner (no sentence wait)

1. **Immediate `text.delta`** — same as today’s Soniox fanout (`trigger_generation: false`, suffix as it grows). No sentence buffer.
2. **`optimize_streaming_latency=2` on this path only** (fallback 400 → 1, already in worker). LiveKit measured ~379 ms TTFB at `2` vs ~741 ms at `0`. Gemini/OpenAI keep the Settings knob as now.
3. **Word-complete hint (Fish-style space)** — if a delta does not already end in whitespace/punctuation, append one space on the wire. Many TTS stacks (and likely xAI’s “enough text”) wait for a complete word. First token `"Hello"` → `"Hello "` can start audio without waiting for token 2. Do not delay for a period.
4. **First PCM out faster** — today’s coalesce (~80 ms / `EL_PCM_COALESCE_MIN_SAMPLES`) holds the first chunk. In chase mode, emit the first `audio.delta` as soon as it is sample-aligned (e.g. ≥10–20 ms), then resume normal coalesce. Do **not** add playback preroll (that would add lag vs translate).
5. **No watchdog `text.done` by default.** A timeout `text.done` would close the utterance and queue Soniox tokens → TTS loses the race. Only consider it later if metrics show **zero** `audio.delta` before `turn_complete` (server not actually incremental).

### B. Smooth while chasing (stutter ≠ extra wait)

1. **Carry leftover odd PCM byte** — do not drop the whole `audio.delta` (Pipecat HTTP note: xAI chops on arbitrary bytes).
2. **Keep mixer `Continuation` on `audio.done`** — already correct; do not `SegmentEnd` between Soniox tokens.
3. **Do not `text.done` on token-gap idle** — already the Soniox fanout policy; chase mode must keep it.
4. Custom playback already uses `clone_outbound` jitter (120 ms) — enough to absorb bursty `audio.delta` without a large preroll.

### C. Revisions

Soniox non-prefix rewrite still `Reset` → `text.clear` then new full text. Unchanged. Optional later: debounce tiny rewrites so a one-token flip does not restart audio; not required for v1 chase.

---

## What we explicitly will not do

- Sentence / clause aggregator before first audio
- `text.done` per Speed sentence (that is Gemini delivery; do not copy onto Soniox)
- Shared mega `tts_delivery`
- Changes to Gemini/OpenAI fanout or EL/Fish workers
- `match AiProvider` in pipeline / mux / engine

---

## Success bar

On Soniox + xAI Custom, same meeting:

- First `audio.delta` tracks first translate tokens (TTFB target: on the order of latency=2 measurements, not “wait for period”).
- Transcript column and voice stay within ~one short phrase; TTS does not wait for a full sentence of translate.
- Gemini/OpenAI + xAI behavior identical to today.
- No odd-PCM dropouts.
- `cargo test --lib`; unit tests for space-hint, leftover byte, chase vs default gate.

If after A–B first audio still only arrives after `text.done` at `turn_complete`, the server is not incremental for this stream — then (and only then) a **first-word kick** `text.done` plus catch-up burst is a fallback, not the default.
