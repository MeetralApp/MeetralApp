import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/shared/ui/button";
import { Badge } from "@/shared/ui/badge";
import { Checkbox } from "@/shared/ui/checkbox";
import { Label } from "@/shared/ui/label";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import { cn } from "@/shared/lib/utils";
import { rangeTrackStyle } from "@/shared/lib/rangeTrackStyle";
import AppTooltip from "@/shared/components/AppTooltip";
import SettingInfoHint from "@/shared/components/SettingInfoHint";
import { settingsFieldLabelClass } from "@/features/config/lib/settingsTypography";

import SetupGuideModal from "@/features/config/components/SetupGuideModal";
import type { ToastType } from "@/shared/context/toastTypes";
import type { AudioDeviceInfo, ConfigView, DeviceRef, DevicesResponse, SaveConfigPayload, SaveConfigResult } from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import { getPlatform } from "@/shared/lib/api/platformApi";
import { previewInboundDucking } from "@/features/pipeline/api/pipelineApi";
import { formatAudioBytes } from "@/features/meeting/library/lib/formatAudioBytes";
import {
  defaultMeetingAudioFolder,
  meetingAudioDiskUsage,
  openMeetingAudioFolder,
  pickMeetingAudioFolder,
} from "@/features/meeting/library/lib/meetingApi";
import {
  AUDIO_ROLE_FIELDS,
  AUDIO_ROLE_HINTS,
  AUDIO_ROLE_LABELS,
  AUDIO_ROLE_ORDER,
  audioDevicesDirty,
  emptyDeviceRef,
  formatDeviceDisplayName,
  formatRoleError,
  groupAudioDevices,
  isRoleRequiredForConfig,
  roleFromValidation,
  suggestVirtualAudioDevices,
  type AudioRole,
  type AudioSetupValidation,
} from "../lib/audioSetup";

const DEFAULT_KEEP_DIRECT_AUDIO = true;
const DEFAULT_ORIGINAL_UNDER = true;
const DEFAULT_ORIGINAL_GAIN = 0.18;
const DUCKING_PREVIEW_DEBOUNCE_MS = 75;
interface Props {
  config: ConfigView;
  devices: AudioDeviceInfo[];
  validation: AudioSetupValidation | null;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onRefreshDevices: () => Promise<DevicesResponse | void>;
  onToast: (type: ToastType, text: string) => void;
  onDirtyChange?: (dirty: boolean) => void;
}

type DraftDevices = Pick<
  ConfigView,
  "userMic" | "teamsMicFeed" | "meetingCapture" | "localPlayback"
>;

const EMPTY_SELECT_VALUE = "__empty__";

const ROLE_FIELD_BY_ROLE = Object.fromEntries(
  AUDIO_ROLE_FIELDS.map((field) => [field.role, field]),
) as Record<AudioRole, (typeof AUDIO_ROLE_FIELDS)[number]>;

function deviceFromSelectValue(
  value: string,
  devices: AudioDeviceInfo[],
): DeviceRef {
  if (!value || value === EMPTY_SELECT_VALUE) return emptyDeviceRef();
  const device = devices.find((d) => d.id === value);
  return device
    ? { id: device.id, name: device.name }
    : { id: value, name: value };
}

function DeviceSelect({
  role,
  value,
  valueLabel,
  devices,
  allowSystemDefault,
  onChange,
}: {
  role: AudioRole;
  value: string;
  valueLabel?: string;
  devices: AudioDeviceInfo[];
  allowSystemDefault: boolean;
  onChange: (device: DeviceRef) => void;
}) {
  const { virtual, physical } = useMemo(
    () => groupAudioDevices(devices),
    [devices],
  );

  const placeholder = allowSystemDefault
    ? "(System default)"
    : "Select a device…";

  const allOptions = useMemo(() => {
    const flat = [...virtual, ...physical];
    if (flat.length === 0) return devices;
    return flat;
  }, [devices, physical, virtual]);

  const selected = allOptions.find((device) => device.id === value);
  const displayLabel = selected
    ? formatDeviceDisplayName(selected.name)
    : value
      ? formatDeviceDisplayName(valueLabel ?? value)
      : placeholder;

  const selectValue = value || EMPTY_SELECT_VALUE;
  const hasGroups = virtual.length > 0 || physical.length > 0;

  return (
    <Select
      value={selectValue}
      onValueChange={(next) =>
        onChange(deviceFromSelectValue(next, devices))
      }
    >
      <AppTooltip label={selected?.name ?? valueLabel ?? ""}>
        <SelectTrigger
          className="w-full"
          aria-label={AUDIO_ROLE_LABELS[role]}
        >
          <SelectValue>{displayLabel}</SelectValue>
        </SelectTrigger>
      </AppTooltip>
      <SelectContent position="popper" className="max-h-60">
        <SelectItem value={EMPTY_SELECT_VALUE}>{placeholder}</SelectItem>
        {hasGroups ? (
          <>
            {virtual.length > 0 && (
              <SelectGroup>
                <SelectLabel>Virtual</SelectLabel>
                {virtual.map((device) => (
                  <SelectItem key={device.id} value={device.id}>
                    {formatDeviceDisplayName(device.name)}
                  </SelectItem>
                ))}
              </SelectGroup>
            )}
            {physical.length > 0 && (
              <SelectGroup>
                <SelectLabel>Physical</SelectLabel>
                {physical.map((device) => (
                  <SelectItem key={device.id} value={device.id}>
                    {formatDeviceDisplayName(device.name)}
                  </SelectItem>
                ))}
              </SelectGroup>
            )}
          </>
        ) : (
          devices.map((device) => (
            <SelectItem key={device.id} value={device.id}>
              {formatDeviceDisplayName(device.name)}
            </SelectItem>
          ))
        )}
      </SelectContent>
    </Select>
  );
}

function RoleField({
  role,
  draft,
  devices,
  config,
  validation,
  onDraftChange,
}: {
  role: AudioRole;
  draft: DraftDevices;
  devices: AudioDeviceInfo[];
  config: ConfigView;
  validation: AudioSetupValidation | null;
  onDraftChange: (field: keyof DraftDevices, device: DeviceRef) => void;
}) {
  const meta = ROLE_FIELD_BY_ROLE[role];
  const entry = roleFromValidation(validation, role);
  const required = isRoleRequiredForConfig(config, role, validation);

  if (!required) {
    const whyHidden =
      role === "teamsMicFeed"
        ? "Not needed while You → Meeting is text-only. Change the live column mode to hear/speak translated audio."
        : role === "localPlayback"
          ? "Not needed while Meeting → You is text-only. Change the live column mode to play translated audio."
          : "Not required for the current live modes.";
    return (
      <div className="flex flex-col gap-1 rounded-md border border-dashed border-border px-3 py-2">
        <span className="text-sm font-medium text-muted-foreground">
          {AUDIO_ROLE_LABELS[role]}
        </span>
        <p className="m-0 text-xs text-muted-foreground">{whyHidden}</p>
      </div>
    );
  }

  const options = devices.filter((d) => d.direction === meta.direction);
  const deviceRef = draft[meta.field];
  const value = deviceRef.id || "";
  const showError = entry && !entry.resolved;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-1.5">
        <Label className={settingsFieldLabelClass}>
          {AUDIO_ROLE_LABELS[role]}
        </Label>
        <SettingInfoHint label={`About ${AUDIO_ROLE_LABELS[role]}`}>
          {AUDIO_ROLE_HINTS[role]}
        </SettingInfoHint>
      </div>
      <DeviceSelect
        role={role}
        value={value}
        valueLabel={deviceRef.name || undefined}
        devices={options}
        allowSystemDefault={meta.allowSystemDefault}
        onChange={(device) => onDraftChange(meta.field, device)}
      />
      {showError && (
        <span
          className="text-sm text-destructive"
          role="alert"
        >
          {formatRoleError(
            role,
            entry.error,
            draft[meta.field].name || entry.resolvedName,
          )}
        </span>
      )}
    </div>
  );
}

export default function AudioDeviceSettings({
  config,
  devices,
  validation,
  onSave,
  onRefreshDevices,
  onToast,
  onDirtyChange,
}: Props) {
  const [draft, setDraft] = useState<DraftDevices>({
    userMic: config.userMic,
    teamsMicFeed: config.teamsMicFeed,
    meetingCapture: config.meetingCapture,
    localPlayback: config.localPlayback,
  });
  const [keepDirectAudio, setKeepDirectAudio] = useState(
    config.keepDirectAudio ?? DEFAULT_KEEP_DIRECT_AUDIO,
  );
  const [saveMeetingAudio, setSaveMeetingAudio] = useState(
    config.saveMeetingAudio ?? false,
  );
  const [saveMeetingAudioFolder, setSaveMeetingAudioFolder] = useState(
    config.saveMeetingAudioFolder ?? "",
  );
  const [defaultAudioFolder, setDefaultAudioFolder] = useState("");
  const [audioDiskBytes, setAudioDiskBytes] = useState<number | null>(null);
  const [originalUnder, setOriginalUnder] = useState(
    config.inboundOriginalUnderTranslation ?? DEFAULT_ORIGINAL_UNDER,
  );
  const [originalGain, setOriginalGain] = useState(
    config.inboundOriginalDuckedGain ?? DEFAULT_ORIGINAL_GAIN,
  );
  const [refreshing, setRefreshing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [guideOpen, setGuideOpen] = useState(false);
  const [platform, setPlatform] = useState("windows");

  useEffect(() => {
    getPlatform()
      .then(setPlatform)
      .catch(() => setPlatform("unknown"));
  }, []);

  useEffect(() => {
    setSaveMeetingAudio(config.saveMeetingAudio ?? false);
    setSaveMeetingAudioFolder(config.saveMeetingAudioFolder ?? "");
  }, [config.saveMeetingAudio, config.saveMeetingAudioFolder]);

  useEffect(() => {
    void defaultMeetingAudioFolder()
      .then(setDefaultAudioFolder)
      .catch(() => setDefaultAudioFolder(""));
  }, []);

  useEffect(() => {
    let cancelled = false;
    void meetingAudioDiskUsage()
      .then((usage) => {
        if (!cancelled) setAudioDiskBytes(usage.totalBytes);
      })
      .catch(() => {
        if (!cancelled) setAudioDiskBytes(null);
      });
    return () => {
      cancelled = true;
    };
  }, [saveMeetingAudio]);

  const saved: DraftDevices = useMemo(
    () => ({
      userMic: config.userMic,
      teamsMicFeed: config.teamsMicFeed,
      meetingCapture: config.meetingCapture,
      localPlayback: config.localPlayback,
    }),
    [config],
  );
  const savedKeepDirect = config.keepDirectAudio ?? DEFAULT_KEEP_DIRECT_AUDIO;
  const savedSaveMeetingAudio = config.saveMeetingAudio ?? false;
  const savedSaveMeetingAudioFolder = config.saveMeetingAudioFolder ?? "";
  const savedOriginalUnder =
    config.inboundOriginalUnderTranslation ?? DEFAULT_ORIGINAL_UNDER;
  const savedOriginalGain =
    config.inboundOriginalDuckedGain ?? DEFAULT_ORIGINAL_GAIN;

  const dirty =
    audioDevicesDirty(saved, draft) ||
    keepDirectAudio !== savedKeepDirect ||
    saveMeetingAudio !== savedSaveMeetingAudio ||
    saveMeetingAudioFolder !== savedSaveMeetingAudioFolder ||
    originalUnder !== savedOriginalUnder ||
    originalGain !== savedOriginalGain;
  const vmSuggestion = useMemo(
    () => suggestVirtualAudioDevices(platform, devices),
    [platform, devices],
  );
  const canSuggestVm =
    Boolean(vmSuggestion.meetingCapture) || Boolean(vmSuggestion.teamsMicFeed);
  const autofillLabel =
    platform === "macos" ? "Auto-fill BlackHole" : "Auto-fill VoiceMeeter";
  const autofillTitle = canSuggestVm
    ? platform === "macos"
      ? "Fill devices from BlackHole names"
      : "Fill devices from VoiceMeeter Banana names"
    : platform === "macos"
      ? "No BlackHole devices found"
      : "No VoiceMeeter devices found";

  useEffect(() => {
    setDraft(saved);
    setKeepDirectAudio(savedKeepDirect);
    setOriginalUnder(savedOriginalUnder);
    setOriginalGain(savedOriginalGain);
  }, [saved, savedKeepDirect, savedOriginalUnder, savedOriginalGain]);

  useEffect(() => {
    onDirtyChange?.(dirty);
  }, [dirty, onDirtyChange]);

  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (previewTimerRef.current) clearTimeout(previewTimerRef.current);
    },
    [],
  );

  const previewDucking = useCallback(
    (enabled: boolean, gain: number, immediate = false) => {
      const run = () => {
        void previewInboundDucking({ enabled, gain }).catch(() => {
        // Inbound may be off — preview is best-effort.
        });
      };
      if (previewTimerRef.current) clearTimeout(previewTimerRef.current);
      if (immediate) {
        run();
        return;
      }
      previewTimerRef.current = setTimeout(run, DUCKING_PREVIEW_DEBOUNCE_MS);
    },
    [],
  );

  const persistDraft = useCallback(async () => {
    setSaving(true);
    try {
      const result = await onSave(
        toSavePayload({
          ...config,
          ...draft,
          keepDirectAudio,
          saveMeetingAudio,
          saveMeetingAudioFolder,
          inboundOriginalUnderTranslation: originalUnder,
          inboundOriginalDuckedGain: originalGain,
        }),
      );
      if (result?.rewired) {
        onToast("success", "Audio devices updated");
      } else {
        onToast("success", "Audio saved");
      }
    } catch (e) {
      onToast("error", String(e));
    } finally {
      setSaving(false);
    }
  }, [
    config,
    draft,
    keepDirectAudio,
    saveMeetingAudio,
    saveMeetingAudioFolder,
    originalUnder,
    originalGain,
    onSave,
    onToast,
  ]);

  const handleDraftChange = (field: keyof DraftDevices, device: DeviceRef) => {
    setDraft((prev) => ({ ...prev, [field]: device }));
  };

  const resetDraft = () => {
    setDraft(saved);
    setKeepDirectAudio(savedKeepDirect);
    setSaveMeetingAudio(savedSaveMeetingAudio);
    setSaveMeetingAudioFolder(savedSaveMeetingAudioFolder);
    setOriginalUnder(savedOriginalUnder);
    setOriginalGain(savedOriginalGain);
    previewDucking(savedOriginalUnder, savedOriginalGain, true);
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={refreshing}
            onClick={async () => {
              setRefreshing(true);
              try {
                await onRefreshDevices();
                onToast("success", "Device list refreshed");
              } catch (e) {
                onToast("error", `Refresh failed: ${String(e)}`);
              } finally {
                setRefreshing(false);
              }
            }}
          >
            {refreshing ? "Refreshing…" : "Refresh"}
          </Button>
          <AppTooltip label={autofillTitle}>
            <span className="inline-flex">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                disabled={!canSuggestVm}
                onClick={() => {
                  setDraft((prev) => ({
                    ...prev,
                    ...(vmSuggestion.meetingCapture
                      ? { meetingCapture: vmSuggestion.meetingCapture }
                      : {}),
                    ...(vmSuggestion.teamsMicFeed
                      ? { teamsMicFeed: vmSuggestion.teamsMicFeed }
                      : {}),
                  }));
                  onToast(
                    "success",
                    platform === "macos"
                      ? "BlackHole suggestions applied"
                      : "VoiceMeeter suggestions applied",
                  );
                }}
              >
                {autofillLabel}
              </Button>
            </span>
          </AppTooltip>
          <Button
            type="button"
            variant="link"
            size="sm"
            className="h-8 px-2"
            onClick={() => setGuideOpen(true)}
          >
            Setup guide
          </Button>
      </div>

      <p className="m-0 text-xs leading-relaxed text-muted-foreground">
        Map each role to the correct Windows or macOS audio device.
      </p>

      <div className="flex flex-col gap-3">
        {AUDIO_ROLE_ORDER.map((role) => (
          <RoleField
            key={role}
            role={role}
            draft={draft}
            devices={devices}
            config={config}
            validation={validation}
            onDraftChange={handleDraftChange}
          />
        ))}
      </div>

      <div className="flex items-start gap-3">
        <Checkbox
          id="keep-direct-audio"
          checked={keepDirectAudio}
          onCheckedChange={(checked) =>
            setKeepDirectAudio(checked === true)
          }
        />
        <span className="inline-flex items-center gap-1.5">
          <Label
            htmlFor="keep-direct-audio"
            className={cn(settingsFieldLabelClass, "cursor-pointer")}
          >
            Keep Direct audio
          </Label>
          <SettingInfoHint label="About Keep Direct audio">
            When translation is off, keep routing meeting capture to your
            headphones (and your mic to the meeting) without using the API. Turn
            off to save CPU. Default: On.
          </SettingInfoHint>
        </span>
      </div>

      <div className="flex flex-col gap-2">
        <div className="flex items-start gap-3">
          <Checkbox
            id="save-meeting-audio"
            checked={saveMeetingAudio}
            onCheckedChange={(checked) =>
              setSaveMeetingAudio(checked === true)
            }
          />
          <span className="inline-flex min-w-0 flex-wrap items-center gap-1.5">
            <Label
              htmlFor="save-meeting-audio"
              className={cn(settingsFieldLabelClass, "cursor-pointer")}
            >
              Save meeting audio
            </Label>
            <SettingInfoHint label="About saving meeting audio">
              When on, records You (mic) and Meeting (speaker) as Opus while a
              live meeting is active (~90–170 MB/h). Playback uses Room mix —
              headphones recommended to avoid echo. Default: Off.
            </SettingInfoHint>
            {!saveMeetingAudio && audioDiskBytes != null && audioDiskBytes > 0 ? (
              <AppTooltip label="Total size of recorded meeting audio on disk">
                <Badge
                  variant="secondary"
                  className="shrink-0 px-1.5 py-0 text-[0.65rem] tabular-nums"
                >
                  {formatAudioBytes(audioDiskBytes)}
                </Badge>
              </AppTooltip>
            ) : null}
          </span>
        </div>
        {saveMeetingAudio ? (
          <div className="ml-7 flex flex-col gap-2">
            <div className="flex min-w-0 items-center gap-2">
              <AppTooltip
                label={
                  saveMeetingAudioFolder.trim() ||
                  defaultAudioFolder ||
                  "App default recordings folder"
                }
                contentClassName="max-w-xs break-all"
              >
                <p className="m-0 min-w-0 flex-1 truncate text-xs text-muted-foreground">
                  {saveMeetingAudioFolder.trim()
                    ? saveMeetingAudioFolder
                    : defaultAudioFolder || "App default recordings folder"}
                </p>
              </AppTooltip>
              {audioDiskBytes != null ? (
                <AppTooltip label="Total size of recorded meeting audio on disk">
                  <Badge
                    variant="secondary"
                    className="shrink-0 px-1.5 py-0 text-[0.65rem] tabular-nums"
                  >
                    {formatAudioBytes(audioDiskBytes)}
                  </Badge>
                </AppTooltip>
              ) : null}
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-8"
                onClick={async () => {
                  try {
                    const picked = await pickMeetingAudioFolder();
                    if (picked) setSaveMeetingAudioFolder(picked);
                  } catch (e) {
                    onToast("error", String(e));
                  }
                }}
              >
                Change folder
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-8"
                onClick={async () => {
                  try {
                    await openMeetingAudioFolder(
                      saveMeetingAudioFolder.trim() || null,
                    );
                  } catch (e) {
                    onToast("error", String(e));
                  }
                }}
              >
                Open
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-8"
                disabled={!saveMeetingAudioFolder.trim()}
                onClick={() => setSaveMeetingAudioFolder("")}
              >
                Reset default
              </Button>
            </div>
          </div>
        ) : null}
      </div>

      <div className="flex flex-col gap-2">
        <div className="flex items-start gap-3">
          <Checkbox
            id="original-under-translation"
            checked={originalUnder}
            onCheckedChange={(checked) => {
              const next = checked === true;
              setOriginalUnder(next);
              previewDucking(next, originalGain, true);
            }}
          />
          <span className="inline-flex items-center gap-1.5">
            <Label
              htmlFor="original-under-translation"
              className={cn(settingsFieldLabelClass, "cursor-pointer")}
            >
              Hear who is speaking (Meeting → You)
            </Label>
            <SettingInfoHint label="About hearing who is speaking">
              Meeting → You only. Plays real meeting voices quietly under the
              translation so you can tell who is talking. Use headphones; don't
              route Playback into From meeting. Default: On.
            </SettingInfoHint>
          </span>
        </div>

        {originalUnder ? (
          <div className="ml-7 flex flex-col gap-1.5">
            <div className="flex items-center justify-between gap-2">
              <Label
                htmlFor="original-under-gain"
                className={settingsFieldLabelClass}
              >
                Quiet volume
              </Label>
              <span className="text-xs text-muted-foreground tabular-nums">
                {Math.round(originalGain * 100)}%
              </span>
            </div>
            <input
              id="original-under-gain"
              type="range"
              min={0}
              max={0.5}
              step={0.01}
              value={originalGain}
              onChange={(e) => {
                const next = Number(e.target.value);
                setOriginalGain(next);
                previewDucking(true, next);
              }}
              style={rangeTrackStyle(originalGain, 0, 0.5)}
              className="h-2 w-full cursor-pointer"
            />
          </div>
        ) : null}
      </div>

      {dirty && (
        <div className="flex items-center justify-end gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={saving}
            onClick={resetDraft}
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

      <SetupGuideModal
        open={guideOpen}
        onClose={() => setGuideOpen(false)}
        platform={platform}
      />
    </div>
  );
}
