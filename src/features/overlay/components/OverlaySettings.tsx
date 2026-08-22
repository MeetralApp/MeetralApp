import { Button } from "@/shared/ui/button";
import { ButtonGroup } from "@/shared/ui/button-group";
import { Checkbox } from "@/shared/ui/checkbox";
import { Label } from "@/shared/ui/label";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import AppTooltip from "@/shared/components/AppTooltip";
import SettingsGroup from "@/shared/components/SettingsGroup";
import { cn } from "@/shared/lib/utils";
import { rangeTrackStyle } from "@/shared/lib/rangeTrackStyle";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";
import { DEFAULT_OVERLAY_SETTINGS } from "@/shared/lib/types/pipeline";
import type { ConfigView, OverlayPosition, OverlaySettings as OverlayConfig, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import type { ToastType } from "@/shared/context/toastTypes";
import { overlayPreview } from "../lib/overlayApi";
import {
  isMac,
  overlayClickThroughHotkey,
  overlayHoldKey,
  overlayToggleHotkey,
} from "../lib/platformLabel";
import OverlayPreviewCard from "./OverlayPreviewCard";
import { useActiveMeeting } from "@/features/meeting/library/hooks/useActiveMeeting";

interface Props {
  config: ConfigView;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast?: (type: ToastType, text: string) => void;
}

/** Short labels so 4 options fit one ButtonGroup row in the Settings drawer. */
const POSITION_OPTIONS: { value: OverlayPosition; label: string }[] = [
  { value: "bottomCenter", label: "Center" },
  { value: "bottomLeft", label: "Left" },
  { value: "bottomRight", label: "Right" },
  { value: "topRight", label: "Top right" },
];

const MODE_OPTIONS: { value: "off" | "on"; label: string }[] = [
  { value: "off", label: "Off" },
  { value: "on", label: "On" },
];

function resolveOverlay(config: ConfigView): OverlayConfig {
  return { ...DEFAULT_OVERLAY_SETTINGS, ...config.overlay };
}

export default function OverlaySettings({
  config,
  onSave,
  onToast,
}: Props) {
  const overlay = resolveOverlay(config);
  const enabled = overlay.enabled;
  const { activeMeeting } = useActiveMeeting();
  const hasCurrentSession = activeMeeting != null;
  const previewDisabled = !enabled || hasCurrentSession;

  const saveOverlay = (patch: Partial<OverlayConfig>) => {
    const next = { ...overlay, ...patch };
    if (patch.showInbound === false && !(patch.showOutbound ?? next.showOutbound)) {
      next.showOutbound = true;
    }
    if (patch.showOutbound === false && !(patch.showInbound ?? next.showInbound)) {
      next.showInbound = true;
    }
    void onSave(toSavePayload({ ...config, overlay: next }));
  };

  const onPreview = async () => {
    if (hasCurrentSession) {
      onToast?.(
        "error",
        "Overlay preview is unavailable while a current session is open",
      );
      return;
    }
    try {
      await overlayPreview();
    } catch (e) {
      onToast?.("error", e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <SettingsGroup>
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-1.5">
            <span className={settingsFieldLabelClass}>Transcript overlay</span>
            <SettingInfoHint label="Transcript overlay">
              Always-on-top window for live translations — toggle anytime with{" "}
              {overlayToggleHotkey} or the tray menu.
            </SettingInfoHint>
          </div>
          <ButtonGroup aria-label="Transcript overlay mode" className="w-full">
            {MODE_OPTIONS.map(({ value, label }) => (
              <Button
                key={value}
                type="button"
                size="sm"
                variant={
                  (value === "on" && enabled) || (value === "off" && !enabled)
                    ? "default"
                    : "secondary"
                }
                aria-pressed={value === "on" ? enabled : !enabled}
                className="h-7 min-w-0 flex-1 px-2 text-xs font-medium"
                onClick={() => saveOverlay({ enabled: value === "on" })}
              >
                {label}
              </Button>
            ))}
          </ButtonGroup>
        </div>

        <div
          className={cn(
            "flex items-start gap-3",
            !enabled && "pointer-events-none opacity-50",
          )}
        >
          <Checkbox
            id="overlay-auto"
            checked={overlay.autoShowWithSession}
            disabled={!enabled}
            onCheckedChange={(v) =>
              saveOverlay({ autoShowWithSession: v === true })
            }
          />
          <div className="flex items-center gap-1.5 min-w-0">
            <Label
              htmlFor="overlay-auto"
              className={cn(
                settingsFieldLabelClass,
                enabled ? "cursor-pointer" : "cursor-default",
              )}
            >
              Show on session start
            </Label>
            <SettingInfoHint label="Show on session start">
              Opens the overlay when a translation session starts. Does not
              hide automatically when idle.
            </SettingInfoHint>
          </div>
        </div>
      </SettingsGroup>

      <div
        className={cn(
          "flex flex-col gap-3",
          !enabled && "pointer-events-none opacity-50",
        )}
        aria-disabled={!enabled}
      >
        <SettingsGroup title="Appearance">
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-1.5">
              <span className={settingsFieldLabelClass}>Position</span>
              <SettingInfoHint label="Overlay position">
                Center / Left / Right sit on the bottom edge of the monitor that
                contains the main window. Top right anchors to the top. Dragging
                the overlay saves an offset until you pick a preset again.
              </SettingInfoHint>
            </div>
            <ButtonGroup aria-label="Overlay position" className="w-full">
              {POSITION_OPTIONS.map(({ value, label }) => (
                <Button
                  key={value}
                  type="button"
                  size="sm"
                  disabled={!enabled}
                  variant={overlay.position === value ? "default" : "secondary"}
                  aria-pressed={overlay.position === value}
                  className="h-7 min-w-0 flex-1 px-2 text-xs font-medium"
                  onClick={() =>
                    saveOverlay({ position: value, offsetX: 0, offsetY: 0 })
                  }
                >
                  {label}
                </Button>
              ))}
            </ButtonGroup>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="flex flex-col gap-1">
              <Label
                htmlFor="overlay-opacity"
                className={cn(settingsFieldLabelClass, "text-xs")}
              >
                Opacity {Math.round(overlay.opacity * 100)}%
              </Label>
              <input
                id="overlay-opacity"
                type="range"
                min={45}
                max={100}
                disabled={!enabled}
                value={Math.round(overlay.opacity * 100)}
                className="w-full cursor-pointer disabled:cursor-not-allowed"
                style={rangeTrackStyle(Math.round(overlay.opacity * 100), 45, 100)}
                aria-valuemin={45}
                aria-valuemax={100}
                aria-valuenow={Math.round(overlay.opacity * 100)}
                onChange={(e) =>
                  saveOverlay({ opacity: Number(e.target.value) / 100 })
                }
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label
                htmlFor="overlay-font"
                className={cn(settingsFieldLabelClass, "text-xs")}
              >
                Font {Math.round(overlay.fontScale * 100)}%
              </Label>
              <input
                id="overlay-font"
                type="range"
                min={85}
                max={135}
                disabled={!enabled}
                value={Math.round(overlay.fontScale * 100)}
                className="w-full cursor-pointer disabled:cursor-not-allowed"
                style={rangeTrackStyle(Math.round(overlay.fontScale * 100), 85, 135)}
                aria-valuemin={85}
                aria-valuemax={135}
                aria-valuenow={Math.round(overlay.fontScale * 100)}
                onChange={(e) =>
                  saveOverlay({ fontScale: Number(e.target.value) / 100 })
                }
              />
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className={settingsFieldLabelClass}>Directions</span>
            <div className="flex flex-wrap gap-x-4 gap-y-2">
              <div className="flex items-center gap-2">
                <Checkbox
                  id="overlay-outbound"
                  checked={overlay.showOutbound}
                  disabled={!enabled}
                  onCheckedChange={(v) =>
                    saveOverlay({ showOutbound: v === true })
                  }
                />
                <Label
                  htmlFor="overlay-outbound"
                  className={cn(
                    settingsFieldLabelClass,
                    enabled ? "cursor-pointer" : "cursor-default",
                  )}
                >
                  You → meeting
                </Label>
              </div>
              <div className="flex items-center gap-2">
                <Checkbox
                  id="overlay-inbound"
                  checked={overlay.showInbound}
                  disabled={!enabled}
                  onCheckedChange={(v) =>
                    saveOverlay({ showInbound: v === true })
                  }
                />
                <Label
                  htmlFor="overlay-inbound"
                  className={cn(
                    settingsFieldLabelClass,
                    enabled ? "cursor-pointer" : "cursor-default",
                  )}
                >
                  Meeting → you
                </Label>
              </div>
            </div>
          </div>
        </SettingsGroup>

        <SettingsGroup title="Behavior">
          <div className="flex items-start gap-3">
            <Checkbox
              id="overlay-click-through"
              checked={overlay.clickThrough}
              disabled={!enabled}
              onCheckedChange={(v) => saveOverlay({ clickThrough: v === true })}
            />
            <div className="flex items-center gap-1.5 min-w-0">
              <Label
                htmlFor="overlay-click-through"
                className={cn(
                  settingsFieldLabelClass,
                  enabled ? "cursor-pointer" : "cursor-default",
                )}
              >
                Click-through
              </Label>
              <SettingInfoHint label="Click-through">
                When on, clicks pass through the overlay. Hold {overlayHoldKey}{" "}
                to drag, mute, or hide; release {overlayHoldKey} to pass through
                again. Resize when click-through is off. Toggle the mode from
                Settings, Tray, or {overlayClickThroughHotkey}.
              </SettingInfoHint>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <Checkbox
              id="overlay-hide-capture"
              checked={overlay.hideFromCapture}
              disabled={!enabled}
              onCheckedChange={(v) =>
                saveOverlay({ hideFromCapture: v === true })
              }
            />
            <div className="flex items-center gap-1.5 min-w-0">
              <Label
                htmlFor="overlay-hide-capture"
                className={cn(
                  settingsFieldLabelClass,
                  enabled ? "cursor-pointer" : "cursor-default",
                )}
              >
                Hide from screen share
              </Label>
              <SettingInfoHint label="Hide from screen share">
                {isMac ? (
                  <>
                    On macOS, hide-from-capture is best-effort only — not a
                    Windows-class guarantee. The overlay may still appear in some
                    screen shares and screenshots. Prefer Share window, or toggle
                    the overlay off.
                  </>
                ) : (
                  <>
                    On Windows the overlay is excluded from Slack, Teams, Zoom
                    screen share and screenshots.
                  </>
                )}
              </SettingInfoHint>
            </div>
          </div>
        </SettingsGroup>

        <SettingsGroup>
          <div className="flex items-center justify-between gap-2">
            <div className="flex min-w-0 items-center gap-1.5">
              <p className={cn("m-0", settingsFieldLabelClass)}>Open overlay</p>
              <SettingInfoHint label="Open overlay">
                Opens the overlay with sample transcript. Unavailable while a
                current session is open.
              </SettingInfoHint>
            </div>
            <AppTooltip
              label={
                hasCurrentSession
                  ? "Unavailable while a current session is open"
                  : ""
              }
            >
              <span className="inline-flex shrink-0">
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  disabled={previewDisabled}
                  onClick={() => void onPreview()}
                >
                  Open
                </Button>
              </span>
            </AppTooltip>
          </div>
          <OverlayPreviewCard
            overlay={overlay}
            transcriptLayout={config.transcriptLayout}
            notesMode={config.sessionMode === "notes"}
          />
        </SettingsGroup>
      </div>
    </div>
  );
}
