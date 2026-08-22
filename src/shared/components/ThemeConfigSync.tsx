import { useEffect, useRef } from "react";

import { useTheme } from "@/shared/context/useTheme";
import { isThemePreference } from "@/shared/lib/theme";

/**
* Syncs AppConfig.themePreference into ThemeProvider when the *saved* config
* value changes (load / after save). Does not re-run when the user optimistically
* changes theme in Settings before save — that was reverting Live UI until reload.
*/
export default function ThemeConfigSync({
  preference,
}: {
  preference?: string | null;
}) {
  const { setPreference } = useTheme();
  const lastConfigPreference = useRef<string | null>(null);

  useEffect(() => {
    if (!isThemePreference(preference)) return;
    if (preference === lastConfigPreference.current) return;
    lastConfigPreference.current = preference;
    setPreference(preference);
  }, [preference, setPreference]);

  return null;
}
