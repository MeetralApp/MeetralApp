import SonioxEngineVoiceSettings from "./SonioxEngineVoiceSettings";
import type { SonioxTtsModelOption } from "@/features/ai/lib/aiTypes";
import type { ConfigView, SaveConfigResult, SonioxVoiceOption } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { ToastType } from "@/shared/context/toastTypes";

/** Meeting → You Engine voice settings (Soniox). */
export default function SonioxInboundVoiceSettings(props: {
  config: ConfigView;
  inboundLocked: boolean;
  ttsModels: SonioxTtsModelOption[];
  sonioxVoices: SonioxVoiceOption[];
  catalogLoading: boolean;
  onRefreshCatalog: () => void;
  onPreviewVoice?: (voice: string, apiKey?: string) => Promise<void>;
  onSave: (
    patch: Parameters<typeof toSavePayload>[1],
  ) => Promise<SaveConfigResult | void>;
  onToast: (type: ToastType, text: string) => void;
}) {
  const { inboundLocked, ...rest } = props;
  return (
    <SonioxEngineVoiceSettings
      {...rest}
      direction="inbound"
      locked={inboundLocked}
    />
  );
}
