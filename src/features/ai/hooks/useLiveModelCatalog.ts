import { useCallback, useEffect, useRef, useState } from "react";

import {
  languagesForLiveModel,
  listSonioxSttModels,
  liveModelsFromConfig,
  persistKeyForProvider,
  seedLiveModelCatalog,
} from "@/features/ai/lib/aiApi";
import type { LiveModelOption } from "@/features/ai/lib/aiTypes";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type {
  ConfigView,
  SaveConfigPayload,
  SaveConfigResult,
} from "@/shared/lib/types/pipeline";
import { withLanguageFlags } from "@/shared/lib/languageFlags";

function enrichLiveModels(list: LiveModelOption[]): LiveModelOption[] {
  return list.map((model) => ({
    ...model,
    languages: withLanguageFlags(model.languages ?? []),
  }));
}

function providerHasKey(config: ConfigView): boolean {
  if (config.aiProvider === "soniox") return Boolean(config.sonioxApiKeyConfigured);
  if (config.aiProvider === "openAi") return Boolean(config.openaiApiKeyConfigured);
  return Boolean(config.geminiApiKeyConfigured ?? config.apiKeyConfigured);
}

/**
* Cache-first live model catalog: Soniox fetches API; Gemini/OpenAI seed static.
* Persists into config like ElevenLabs models / Soniox voices.
*/
export function useLiveModelCatalog(
  config: ConfigView,
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>,
  refreshNonce = 0,
) {
  const provider = config.aiProvider;
  const cached = enrichLiveModels(liveModelsFromConfig(config, provider));
  const [models, setModels] = useState<LiveModelOption[]>(cached);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const lastNonceRef = useRef(refreshNonce);
  const onSaveRef = useRef(onSave);
  onSaveRef.current = onSave;
  const configRef = useRef(config);
  configRef.current = config;

  const persist = useCallback(async (list: LiveModelOption[]) => {
    const key = persistKeyForProvider(provider);
    await onSaveRef.current(
      toSavePayload(configRef.current, { [key]: list }),
    );
  }, [provider]);

  const refresh = useCallback(
    async (opts?: { silent?: boolean }) => {
      setLoading(true);
      if (!opts?.silent) setError(null);
      try {
        let list: LiveModelOption[];
        if (provider === "soniox" && providerHasKey(configRef.current)) {
          try {
            list = await listSonioxSttModels();
            if (list.length === 0) {
              list = await seedLiveModelCatalog(provider);
            }
          } catch (e) {
            list = await seedLiveModelCatalog(provider);
            if (!opts?.silent) setError(String(e));
          }
        } else {
          list = await seedLiveModelCatalog(provider);
        }
        const enriched = enrichLiveModels(list);
        setModels(enriched);
        await persist(enriched);
      } catch (e) {
        if (!opts?.silent) setError(String(e));
      } finally {
        setLoading(false);
      }
    },
    [persist, provider],
  );

  useEffect(() => {
    const nonceChanged = lastNonceRef.current !== refreshNonce;
    lastNonceRef.current = refreshNonce;
    const fromConfig = enrichLiveModels(
      liveModelsFromConfig(configRef.current, provider),
    );
    if (!nonceChanged && fromConfig.length > 0) {
      setModels(fromConfig);
      return;
    }
    void refresh({ silent: true });
  }, [provider, refreshNonce, refresh]);

  const languages = withLanguageFlags(
    languagesForLiveModel(models, config.liveModel),
  );

  return {
    models,
    languages,
    loading,
    error,
    refresh,
  };
}
