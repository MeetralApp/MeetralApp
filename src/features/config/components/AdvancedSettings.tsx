import { useCallback, useEffect, useMemo, useState } from "react";

import { Button } from "@/shared/ui/button";
import { ButtonGroup } from "@/shared/ui/button-group";
import { Checkbox } from "@/shared/ui/checkbox";
import { Label } from "@/shared/ui/label";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import SettingsGroup from "@/shared/components/SettingsGroup";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import { cn } from "@/shared/lib/utils";

import type { ConfigView, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { ToastType } from "@/shared/context/toastTypes";
import { useTheme } from "@/shared/context/useTheme";
import type { ThemePreference } from "@/shared/lib/theme";

const THEME_OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "dark", label: "Dark" },
  { value: "light", label: "Light" },
  { value: "system", label: "System" },
];

const AUTO_END_MEETING_AFTER_MIN_PRESETS = [1, 5, 10, 15] as const;
type AutoEndMeetingAfterMin = (typeof AUTO_END_MEETING_AFTER_MIN_PRESETS)[number];

type AutoEndSelection = "off" | AutoEndMeetingAfterMin;

const AUTO_END_OPTIONS: { value: AutoEndSelection; label: string }[] = [
  { value: "off", label: "Off" },
  { value: 1, label: "1 min" },
  { value: 5, label: "5" },
  { value: 10, label: "10" },
  { value: 15, label: "15" },
];

interface Props {
  config: ConfigView;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast: (type: ToastType, text: string) => void;
  onDirtyChange?: (dirty: boolean) => void;
}

type Draft = {
  closeToTray: boolean;
  themePreference: ThemePreference;
  autoEndMeeting: boolean;
  autoEndMeetingAfterMin: AutoEndMeetingAfterMin;
};

const DEFAULT_CLOSE_TO_TRAY = true;
const DEFAULT_AUTO_END_MEETING = false;
const DEFAULT_AUTO_END_MEETING_AFTER_MIN: AutoEndMeetingAfterMin = 5;

function formatOnOff(value: boolean): string {
  return value ? "On" : "Off";
}

function snapAutoEndMinutes(value: number): AutoEndMeetingAfterMin {
  if (!Number.isFinite(value)) return DEFAULT_AUTO_END_MEETING_AFTER_MIN;
  return AUTO_END_MEETING_AFTER_MIN_PRESETS.reduce((best, preset) =>
    Math.abs(preset - value) < Math.abs(best - value) ? preset : best,
  );
}

function draftFromConfig(config: ConfigView): Draft {
  return {
    closeToTray: config.closeToTray ?? true,
    themePreference: config.themePreference ?? "dark",
    autoEndMeeting: config.autoEndMeeting ?? DEFAULT_AUTO_END_MEETING,
    autoEndMeetingAfterMin: snapAutoEndMinutes(
      config.autoEndMeetingAfterMin ?? DEFAULT_AUTO_END_MEETING_AFTER_MIN,
    ),
  };
}

function autoEndSelection(draft: Draft): AutoEndSelection {
  return draft.autoEndMeeting ? draft.autoEndMeetingAfterMin : "off";
}

/** Theme and auto-end auto-save on select — exclude them so Save doesn't flash. */
function isAppDraftDirty(draft: Draft, saved: Draft): boolean {
  return draft.closeToTray !== saved.closeToTray;
}

export default function AdvancedSettings({
  config,
  onSave,
  onToast,
  onDirtyChange,
}: Props) {
  const { setPreference: setThemePreference } = useTheme();
  const saved = useMemo(() => draftFromConfig(config), [config]);
  const [draft, setDraft] = useState(saved);
  const [saving, setSaving] = useState(false);

  const dirty = isAppDraftDirty(draft, saved);

  useEffect(() => {
    setDraft(saved);
  }, [saved]);

  useEffect(() => {
    onDirtyChange?.(dirty);
  }, [dirty, onDirtyChange]);

  const persistDraft = useCallback(async () => {
    setSaving(true);
    try {
      setThemePreference(draft.themePreference);
      await onSave(
        toSavePayload({
          ...config,
          closeToTray: draft.closeToTray,
          themePreference: draft.themePreference,
          autoEndMeeting: draft.autoEndMeeting,
          autoEndMeetingAfterMin: draft.autoEndMeetingAfterMin,
        }),
      );
      onToast("success", "Settings saved");
    } catch (e) {
      onToast("error", String(e));
    } finally {
      setSaving(false);
    }
  }, [config, draft, onSave, onToast, setThemePreference]);

  const saveThemePreference = useCallback(
    (themePreference: ThemePreference) => {
      if (themePreference === (config.themePreference ?? "dark")) return;
      setDraft((d) => ({ ...d, themePreference }));
      setThemePreference(themePreference);
      void onSave(toSavePayload({ ...config, themePreference })).catch((e) => {
        const fallback = config.themePreference ?? "dark";
        setDraft((d) => ({ ...d, themePreference: fallback }));
        setThemePreference(fallback);
        onToast("error", String(e));
      });
    },
    [config, onSave, onToast, setThemePreference],
  );

  const saveAutoEnd = useCallback(
    (selection: AutoEndSelection) => {
      const next: Pick<Draft, "autoEndMeeting" | "autoEndMeetingAfterMin"> =
        selection === "off"
          ? {
              autoEndMeeting: false,
              autoEndMeetingAfterMin: draft.autoEndMeetingAfterMin,
            }
          : { autoEndMeeting: true, autoEndMeetingAfterMin: selection };
      if (
        next.autoEndMeeting === draft.autoEndMeeting &&
        next.autoEndMeetingAfterMin === draft.autoEndMeetingAfterMin
      ) {
        return;
      }
      setDraft((d) => ({ ...d, ...next }));
      void onSave(
        toSavePayload({
          ...config,
          autoEndMeeting: next.autoEndMeeting,
          autoEndMeetingAfterMin: next.autoEndMeetingAfterMin,
        }),
      ).catch((e) => {
        setDraft((d) => ({
          ...d,
          autoEndMeeting: saved.autoEndMeeting,
          autoEndMeetingAfterMin: saved.autoEndMeetingAfterMin,
        }));
        onToast("error", String(e));
      });
    },
    [config, draft, onSave, onToast, saved],
  );

  const selectedAutoEnd = autoEndSelection(draft);

  return (
    <div className="flex flex-col gap-3">
      <SettingsGroup
        title="Theme"
        titleHint={
          <SettingInfoHint label="About Theme">
            Default is Dark. System follows your OS appearance.
          </SettingInfoHint>
        }
      >
        <ButtonGroup aria-label="Theme" className="w-full max-w-xs">
          {THEME_OPTIONS.map(({ value, label }) => (
            <Button
              key={value}
              type="button"
              size="sm"
              variant={
                draft.themePreference === value ? "default" : "secondary"
              }
              aria-pressed={draft.themePreference === value}
              className="h-7 min-w-0 flex-1 px-2 text-xs font-medium"
              onClick={() => saveThemePreference(value)}
            >
              {label}
            </Button>
          ))}
        </ButtonGroup>
      </SettingsGroup>

      <SettingsGroup title="Application">
        <div className="flex items-start gap-3">
          <Checkbox
            id="close-to-tray"
            checked={draft.closeToTray}
            onCheckedChange={(checked) =>
              setDraft((d) => ({ ...d, closeToTray: checked === true }))
            }
          />
          <span className="inline-flex items-center gap-1.5">
            <Label
              htmlFor="close-to-tray"
              className={cn(settingsFieldLabelClass, "cursor-pointer")}
            >
              Minimize to tray when closing window
            </Label>
            <SettingInfoHint label="About Minimize to tray">
              Hides the app to the system tray instead of quitting when you close
              the window. Default: {formatOnOff(DEFAULT_CLOSE_TO_TRAY)}.
            </SettingInfoHint>
          </span>
        </div>

        <div className="flex flex-col gap-2">
          <span className="inline-flex items-center gap-1.5">
            <span className={settingsFieldLabelClass}>Auto-end meeting</span>
            <SettingInfoHint label="About Auto-end meeting">
              Ends the live meeting when no new transcript is saved for this
              long — including Direct with no translation. Default: Off.
            </SettingInfoHint>
          </span>
          <ButtonGroup aria-label="Auto-end meeting" className="w-full">
            {AUTO_END_OPTIONS.map(({ value, label }) => (
              <Button
                key={String(value)}
                type="button"
                size="sm"
                variant={selectedAutoEnd === value ? "default" : "secondary"}
                aria-pressed={selectedAutoEnd === value}
                className="h-7 min-w-0 flex-1 px-2 text-xs font-medium"
                onClick={() => saveAutoEnd(value)}
              >
                {label}
              </Button>
            ))}
          </ButtonGroup>
        </div>
      </SettingsGroup>

      {dirty && (
        <div className="flex items-center justify-end gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={saving}
            onClick={() => setDraft(saved)}
          >
            Cancel
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={saving}
            onClick={() => void persistDraft()}
          >
            {saving ? "Saving…" : "Save"}
          </Button>
        </div>
      )}
    </div>
  );
}
