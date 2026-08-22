import SonioxEngineVoiceSettings from "./SonioxEngineVoiceSettings";
import type { SonioxTtsModelOption } from "@/features/ai/lib/aiTypes";
import type { ConfigView, SaveConfigPayload, SaveConfigResult, SonioxVoiceOption } from "@/shared/lib/types/pipeline";
import type { ToastType } from "@/shared/context/toastTypes";

/** Meeting → You Engine voice settings (Soniox). */
export default function SonioxInboundVoiceSettings(props: {
  config: ConfigView;
  inboundLocked: boolean;
  ttsModels: SonioxTtsModelOption[];
  sonioxVoices: SonioxVoiceOption[];
  catalogLoading: boolean;
  showSharedModel?: boolean;
  onRefreshCatalog: () => void;
  onPreviewVoice?: (voice: string, apiKey?: string) => Promise<void>;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
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
