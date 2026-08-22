import SettingInfoHint from "@/shared/components/SettingInfoHint";
import type { AiProvider } from "@/features/ai/lib/aiTypes";
import {
  apiKeyHintBody,
  apiKeyHintLabel,
  liveModelHintBody,
} from "@/features/config/lib/settingsDrawerStatus";

export function EngineTitleHint() {
  return (
    <SettingInfoHint label="About engine">
      Choose the live speech translation engine. Voice, audio, and app
      preferences are on the other Settings tabs.
    </SettingInfoHint>
  );
}

export function ApiKeyTitleHint({ provider }: { provider: AiProvider }) {
  return (
    <SettingInfoHint label={apiKeyHintLabel(provider)}>
      {apiKeyHintBody(provider)}
    </SettingInfoHint>
  );
}

export function LiveModelTitleHint({
  provider,
  sessionMode,
}: {
  provider: AiProvider;
  sessionMode?: "interpreter" | "notes";
}) {
  return (
    <SettingInfoHint
      label={
        sessionMode === "notes" && provider === "openAi"
          ? "About STT model"
          : "About live model"
      }
    >
      {liveModelHintBody(provider, sessionMode)}
    </SettingInfoHint>
  );
}

export function SonioxContextTitleHint() {
  return (
    <SettingInfoHint label="About context">
      Improves recognition and translation for You and Meeting (Soniox only).
      Four sections: meeting facts, background text, words to recognize, and
      preferred translations. Applies next time you start translate.
    </SettingInfoHint>
  );
}
