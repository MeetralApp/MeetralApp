import { useEffect, useState } from "react";

import { getAiCatalog } from "../lib/aiApi";
import type { AiProvider, ProviderCatalog } from "../lib/aiTypes";
import { FALLBACK_LANGUAGES } from "@/features/pipeline/lib/languages";

const cache = new Map<AiProvider, ProviderCatalog>();
const inflight = new Map<AiProvider, Promise<ProviderCatalog>>();

function fallbackCatalog(provider: AiProvider): ProviderCatalog {
  const languages = FALLBACK_LANGUAGES.map((lang) => ({
    code: lang.code,
    name: lang.name,
    countryCode: lang.countryCode,
  }));

  if (provider === "soniox") {
    return {
      provider,
      languages,
      liveModels: [
        {
          id: "stt-rt-v5",
          label: "Soniox STT RT v5",
          description: "Real-time speech-to-text translation over Soniox WebSocket.",
        },
      ],
      summaryModels: [
        {
          id: "gemini-2.5-flash",
          label: "Summary via Gemini / OpenAI",
          description:
            "Soniox has no summary LLM. Add a Gemini or OpenAI key to enable summaries.",
        },
      ],
      defaults: {
        liveModel: "stt-rt-v5",
        summaryModel: "gemini-2.5-flash",
      },
      capabilities: {
        supportsVadConfig: false,
        supportsEchoTargetLanguage: false,
        liveUploadSampleRate: 16000,
        supportsNativeSummary: false,
        usesSeparateTts: true,
        supportsNotesSttOnly: true,
      },
    };
  }

  if (provider === "openAi") {
    const openAiCodes = new Set([
      "en", "es", "pt", "fr", "ja", "ru", "zh", "de", "ko", "hi", "id", "vi", "it",
    ]);
    return {
      provider,
      languages: languages.filter((lang) => openAiCodes.has(lang.code)),
      liveModels: [
        {
          id: "gpt-realtime-translate",
          label: "GPT Realtime Translate",
          description: "OpenAI live speech translation over WebSocket.",
        },
      ],
      summaryModels: [
        {
          id: "gpt-4o",
          label: "GPT-4o",
          description: "High-quality meeting summaries with JSON output.",
        },
        {
          id: "gpt-4.1-mini",
          label: "GPT-4.1 Mini",
          description: "Faster summaries for long transcripts.",
        },
      ],
      defaults: {
        liveModel: "gpt-realtime-translate",
        summaryModel: "gpt-4o",
      },
      capabilities: {
        supportsVadConfig: false,
        supportsEchoTargetLanguage: false,
        liveUploadSampleRate: 24000,
        supportsNotesSttOnly: true,
      },
      notesSttModel: {
        id: "gpt-realtime-whisper",
        label: "GPT Realtime Whisper",
        description:
          "OpenAI Realtime transcription for Notes (STT-only; no translation session).",
      },
    };
  }

  return {
    provider,
    languages,
    liveModels: [
      {
        id: "gemini-3.5-live-translate-preview",
        label: "Gemini 3.5 Live Translate",
        description: "Real-time speech translation over Gemini Live WebSocket.",
      },
    ],
    summaryModels: [
      {
        id: "gemini-2.5-flash",
        label: "Gemini 2.5 Flash",
        description: "Fast REST summarization with JSON output.",
      },
      {
        id: "gemini-2.0-flash",
        label: "Gemini 2.0 Flash",
        description: "Stable flash model for meeting summaries.",
      },
    ],
    defaults: {
      liveModel: "gemini-3.5-live-translate-preview",
      summaryModel: "gemini-2.5-flash",
    },
    capabilities: {
      supportsVadConfig: true,
      supportsEchoTargetLanguage: true,
      liveUploadSampleRate: 16000,
      supportsNotesSttOnly: false,
    },
  };
}

export function loadAiCatalog(provider: AiProvider): Promise<ProviderCatalog> {
  const cached = cache.get(provider);
  if (cached) {
    return Promise.resolve(cached);
  }
  const pending = inflight.get(provider);
  if (pending) {
    return pending;
  }
  const promise = getAiCatalog(provider)
    .then((catalog) => {
      cache.set(provider, catalog);
      return catalog;
    })
    .catch(() => {
      const fallback = fallbackCatalog(provider);
      cache.set(provider, fallback);
      return fallback;
    })
    .finally(() => {
      inflight.delete(provider);
    });
  inflight.set(provider, promise);
  return promise;
}

export function invalidateAiCatalog(provider?: AiProvider) {
  if (provider) {
    cache.delete(provider);
    return;
  }
  cache.clear();
}

export function useAiCatalog(provider: AiProvider) {
  const [catalog, setCatalog] = useState<ProviderCatalog | null>(
    cache.get(provider) ?? null,
  );

  useEffect(() => {
    const cached = cache.get(provider);
    if (cached) {
      setCatalog(cached);
      return;
    }
    let cancelled = false;
    void loadAiCatalog(provider).then((next) => {
      if (!cancelled) {
        setCatalog(next);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [provider]);

  return catalog ?? fallbackCatalog(provider);
}
