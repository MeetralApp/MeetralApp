/** Primary Settings drawer tabs. */
export type SettingsTab =
  | "translate"
  | "intelligence"
  | "voice"
  | "audio"
  | "app";

/**
* Deep-link targets for opening Settings.
* Legacy values (`api`, `provider`, `languages`) map onto the Translate tab;
* `summaries` / `summary` map onto the Intelligence tab.
*/
export type SettingsFocus =
  | SettingsTab
  | "api"
  | "languages"
  | "provider"
  | "sonioxContext"
  | "customVoice"
  | "summaries"
  | "summary"
  | "overlay";

export function resolveSettingsTab(
  focus: SettingsFocus | null | undefined,
): SettingsTab {
  switch (focus) {
    case "voice":
    case "customVoice":
      return "voice";
    case "audio":
      return "audio";
    case "app":
    case "overlay":
      return "app";
    case "intelligence":
    case "summaries":
    case "summary":
      return "intelligence";
    case "translate":
    case "api":
    case "languages":
    case "provider":
    case "sonioxContext":
    default:
      return "translate";
  }
}
