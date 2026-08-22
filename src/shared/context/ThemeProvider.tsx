import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import {
  applyThemePreference,
  isThemePreference,
  readStoredThemePreference,
  resolveTheme,
  type ResolvedTheme,
  type ThemePreference,
} from "@/shared/lib/theme";
import { ThemeContext } from "./themeContext";

export function ThemeProvider({
  preference: preferenceFromConfig,
  children,
}: {
  /** When config loads, sync preference from AppConfig. */
  preference?: ThemePreference | null;
  children: ReactNode;
}) {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    readStoredThemePreference(),
  );
  const [resolved, setResolved] = useState<ResolvedTheme>(() =>
    resolveTheme(readStoredThemePreference()),
  );

  const setPreference = useCallback((next: ThemePreference) => {
    setPreferenceState(next);
    setResolved(applyThemePreference(next));
  }, []);

  // Only apply when the *config prop* changes — not when local preference changes
  // (optimistic Settings preview would otherwise be overwritten).
  const lastConfigPreference = useRef<ThemePreference | null>(null);
  useEffect(() => {
    if (!isThemePreference(preferenceFromConfig)) return;
    if (preferenceFromConfig === lastConfigPreference.current) return;
    lastConfigPreference.current = preferenceFromConfig;
    setPreference(preferenceFromConfig);
  }, [preferenceFromConfig, setPreference]);

  useEffect(() => {
    if (preference !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => setResolved(applyThemePreference("system"));
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [preference]);

  const value = useMemo(
    () => ({ preference, resolved, setPreference }),
    [preference, resolved, setPreference],
  );

  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}
