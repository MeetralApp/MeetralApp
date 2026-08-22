import { useCallback } from "react";

import SecretApiKeyField from "@/shared/components/SecretApiKeyField";

import type { ToastType } from "@/shared/context/toastTypes";
import type { ConfigView, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";

export interface ApiKeySettingsState {
  dirty: boolean;
}

interface Props {
  config: ConfigView;
  locked?: boolean;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onTest: (apiKey: string) => Promise<void>;
  onToast: (type: ToastType, text: string) => void;
  onStateChange?: (state: ApiKeySettingsState) => void;
}

function keyOptionsForProvider(
  provider: ConfigView["aiProvider"],
  apiKey: string,
): {
  geminiApiKey?: string;
  openaiApiKey?: string;
  sonioxApiKey?: string;
} {
  if (provider === "openAi") return { openaiApiKey: apiKey };
  if (provider === "soniox") return { sonioxApiKey: apiKey };
  return { geminiApiKey: apiKey };
}

function clearOptionsForProvider(provider: ConfigView["aiProvider"]): {
  clearGeminiApiKey?: boolean;
  clearOpenaiApiKey?: boolean;
  clearSonioxApiKey?: boolean;
} {
  if (provider === "openAi") return { clearOpenaiApiKey: true };
  if (provider === "soniox") return { clearSonioxApiKey: true };
  return { clearGeminiApiKey: true };
}

export default function ApiKeySettings({
  config,
  locked = false,
  onSave,
  onTest,
  onToast,
  onStateChange,
}: Props) {
  const persistKey = useCallback(
    async (apiKey: string) => {
      await onSave(
        toSavePayload(config, keyOptionsForProvider(config.aiProvider, apiKey)),
      );
      onToast("success", "API key saved");
    },
    [config, onSave, onToast],
  );

  const removeKey = useCallback(async () => {
    await onSave(
      toSavePayload(config, clearOptionsForProvider(config.aiProvider)),
    );
    onToast("success", "API key removed");
  }, [config, onSave, onToast]);

  return (
    <SecretApiKeyField
      label="API key"
      showLabel={false}
      configured={config.apiKeyConfigured}
      locked={locked}
      onSave={persistKey}
      onTest={onTest}
      onRemove={removeKey}
      onDirtyChange={onStateChange}
    />
  );
}
