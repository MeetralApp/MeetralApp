import { useCallback } from "react";

import SecretApiKeyField from "@/shared/components/SecretApiKeyField";

import type { ToastType } from "@/shared/context/toastTypes";
import type { ConfigView, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";

export interface FishAudioApiKeySettingsState {
  dirty: boolean;
}

interface Props {
  config: ConfigView;
  locked?: boolean;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onTest: (apiKey: string) => Promise<void>;
  onToast: (type: ToastType, text: string) => void;
  onStateChange?: (state: FishAudioApiKeySettingsState) => void;
  onKeySaved?: () => void;
}

export default function FishAudioApiKeySettings({
  config,
  locked = false,
  onSave,
  onTest,
  onToast,
  onStateChange,
  onKeySaved,
}: Props) {
  const persistKey = useCallback(
    async (apiKey: string) => {
      await onSave(toSavePayload(config, { fishaudioApiKey: apiKey }));
      onToast("success", "Fish Audio API key saved");
      onKeySaved?.();
    },
    [config, onKeySaved, onSave, onToast],
  );

  const removeKey = useCallback(async () => {
    await onSave(toSavePayload(config, { clearFishaudioApiKey: true }));
    onToast("success", "Fish Audio API key removed");
  }, [config, onSave, onToast]);

  return (
    <SecretApiKeyField
      label="API key"
      configured={Boolean(config.fishaudioApiKeyConfigured)}
      locked={locked}
      showLabel={false}
      onSave={persistKey}
      onTest={onTest}
      onRemove={removeKey}
      onDirtyChange={onStateChange}
      onSaved={onKeySaved}
    />
  );
}
