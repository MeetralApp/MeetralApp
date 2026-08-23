import { useCallback, useMemo, useState } from "react";
import {
  ArrowLeftRight,
  ChevronRight,
  Sparkles,
} from "lucide-react";

import FlagIcon from "@/features/pipeline/components/FlagIcon";
import InlineTextForm from "@/features/pipeline/components/InlineTextForm";
import { useMeetingElapsed } from "@/features/pipeline/hooks/useMeetingElapsed";
import { getLanguageByCode } from "@/features/pipeline/lib/languages";
import { meetingChipDisplayTitle } from "@/features/meeting/library/lib/meetingDisplay";
import type { MeetingRecord } from "@/features/meeting/library/lib/meetingTypes";
import ConfirmDialog from "@/shared/components/ConfirmDialog";
import type { ToastType } from "@/shared/context/toastTypes";
import { headerChipTriggerClass } from "@/shared/lib/headerChrome";
import { cn } from "@/shared/lib/utils";
import type {
  ConfigView,
  SaveConfigPayload,
  SaveConfigResult,
  SessionMode,
} from "@/shared/lib/types/pipeline";
import { toSavePayload } from "@/features/pipeline/lib/toSavePayload";
import { useAiCatalog } from "@/features/ai/hooks/useAiCatalog";
import { isSonioxContextOverBudget } from "@/features/voice/lib/sonioxContext";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { Label } from "@/shared/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/shared/ui/select";
import { Separator } from "@/shared/ui/separator";
import AppTooltip from "@/shared/components/AppTooltip";

const ALWAYS_ON_VALUE = "__always_on__";

/** Side cells (elapsed · context) share one width so the chip stays symmetric. */
const CHIP_SIDE_CELL_CLASS =
  "flex w-14 shrink-0 items-center justify-center";
const CHIP_SIDE_ICON_CLASS = "size-3.5 shrink-0";

type Props = {
  meeting: MeetingRecord | null;
  myLanguage: string;
  meetingLanguage: string;
  languagesLocked?: boolean;
  showSonioxContext: boolean;
  config: ConfigView;
  contextDisabled?: boolean;
  onRename: (title: string) => Promise<void>;
  onEnd: () => Promise<void>;
  onOpenLanguages: () => void;
  onSave: (payload: SaveConfigPayload) => Promise<SaveConfigResult | void>;
  onToast?: (type: ToastType, text: string) => void;
};

function contextLabel(config: ConfigView): string {
  const profiles = config.sonioxContextProfiles ?? [];
  const activeId = config.sonioxActiveContextProfileId ?? null;
  if (!activeId) return "Always-on only";
  const profile = profiles.find((p) => p.id === activeId);
  if (!profile) return "Always-on only";
  return profile.includeAlwaysOn
    ? `${profile.name} (+ Always-on)`
    : profile.name;
}

/**
* Live header session hub — single-line chip + popover for meeting / languages / context.
*/
export default function SessionHeaderHub({
  meeting,
  myLanguage,
  meetingLanguage,
  languagesLocked = false,
  showSonioxContext,
  config,
  contextDisabled = false,
  onRename,
  onEnd,
  onOpenLanguages,
  onSave,
  onToast,
}: Props) {
  const [open, setOpen] = useState(false);
  const [hovering, setHovering] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [endConfirmOpen, setEndConfirmOpen] = useState(false);
  const [savingContext, setSavingContext] = useState(false);
  const [savingSessionMode, setSavingSessionMode] = useState(false);
  const catalog = useAiCatalog(config.aiProvider);
  const notesCapable = catalog.capabilities.supportsNotesSttOnly === true;

  const sessionMode: SessionMode = config.sessionMode ?? "interpreter";
  const notesMode = sessionMode === "notes";
  const sessionModeLocked =
    languagesLocked || meeting?.status === "live";

  const saveSessionMode = useCallback(
    async (next: SessionMode) => {
      if (next === sessionMode || sessionModeLocked || savingSessionMode) return;
      if (next === "notes" && !notesCapable) {
        onToast?.(
          "error",
          "Notes mode needs Soniox or OpenAI (STT-only). Switch AI provider first.",
        );
        return;
      }
      setSavingSessionMode(true);
      try {
        const leavingNotes = sessionMode === "notes";
        const patch: Partial<ConfigView> = {
          sessionMode: next,
        };
        if (leavingNotes) {
          // Capture Notes language, then restore Interpreter stash → active so
          // toSavePayload does not mirror Notes-clamped originalAudio/providerNative
          // into the interpreter mode/voice stash (would wipe Custom/Translate).
          patch.notesLanguage = config.myLanguage;
          patch.myLanguage =
            config.interpreterMyLanguage ?? config.myLanguage;
          patch.meetingLanguage =
            config.interpreterMeetingLanguage ?? config.meetingLanguage;
          patch.outboundMode =
            config.interpreterOutboundMode ?? config.outboundMode;
          patch.inboundMode =
            config.interpreterInboundMode ?? config.inboundMode;
          patch.outboundVoiceOutput =
            config.interpreterOutboundVoiceOutput ??
            config.outboundVoiceOutput ??
            "providerNative";
          patch.inboundVoiceOutput =
            config.interpreterInboundVoiceOutput ??
            config.inboundVoiceOutput ??
            "providerNative";
          patch.interpreterMyLanguage =
            config.interpreterMyLanguage ?? config.myLanguage;
          patch.interpreterMeetingLanguage =
            config.interpreterMeetingLanguage ?? config.meetingLanguage;
          patch.interpreterOutboundMode =
            config.interpreterOutboundMode ?? config.outboundMode;
          patch.interpreterInboundMode =
            config.interpreterInboundMode ?? config.inboundMode;
          patch.interpreterOutboundVoiceOutput =
            config.interpreterOutboundVoiceOutput ??
            config.outboundVoiceOutput ??
            "providerNative";
          patch.interpreterInboundVoiceOutput =
            config.interpreterInboundVoiceOutput ??
            config.inboundVoiceOutput ??
            "providerNative";
        } else {
          // Capture Interpreter pair + pipeline prefs before Notes projection.
          patch.interpreterMyLanguage = config.myLanguage;
          patch.interpreterMeetingLanguage = config.meetingLanguage;
          patch.interpreterOutboundMode = config.outboundMode;
          patch.interpreterInboundMode = config.inboundMode;
          patch.interpreterOutboundVoiceOutput =
            config.outboundVoiceOutput ?? "providerNative";
          patch.interpreterInboundVoiceOutput =
            config.inboundVoiceOutput ?? "providerNative";
        }
        await onSave(toSavePayload({ ...config, ...patch }));
        onToast?.(
          "success",
          next === "notes"
            ? "Notes mode — same-language capture (STT only)."
            : "Interpreter mode — cross-language translate.",
        );
      } catch (err) {
        onToast?.(
          "error",
          err instanceof Error ? err.message : String(err),
        );
      } finally {
        setSavingSessionMode(false);
      }
    },
    [
      config,
      notesCapable,
      onSave,
      onToast,
      savingSessionMode,
      sessionMode,
      sessionModeLocked,
    ],
  );

  const my = getLanguageByCode(myLanguage);
  const meetingLang = getLanguageByCode(meetingLanguage);
  const myName = my?.name ?? myLanguage;
  const meetingName = meetingLang?.name ?? meetingLanguage;
  const langPairLabel = notesMode ? myName : `${myName} → ${meetingName}`;
  const capturingNotes = notesMode && languagesLocked;
  const displayTitle = meeting ? meetingChipDisplayTitle(meeting) : null;
  const meetingElapsed = useMeetingElapsed(
    meeting?.status === "live" ? meeting.startedAtMs : null,
    meeting?.endedAtMs ?? null,
  );
  const ctxLabel = showSonioxContext ? contextLabel(config) : null;
  const overBudget = useMemo(
    () => (showSonioxContext ? isSonioxContextOverBudget(config) : false),
    [config, showSonioxContext],
  );
  const profiles = config.sonioxContextProfiles ?? [];
  const activeId = config.sonioxActiveContextProfileId ?? null;
  const selectValue = activeId || ALWAYS_ON_VALUE;

  const tooltipParts = [
    displayTitle ? (meeting?.title ?? displayTitle) : null,
    meetingElapsed.display ? `Elapsed ${meetingElapsed.display}` : null,
    capturingNotes ? "Capturing notes (STT)" : null,
    languagesLocked
      ? notesMode
        ? `Language locked · ${myName}`
        : `Languages locked · ${myName} → ${meetingName}`
      : langPairLabel,
    ctxLabel
      ? overBudget
        ? `${ctxLabel} — Context over budget`
        : ctxLabel
      : null,
  ].filter(Boolean);

  const ariaLabel = [
    displayTitle ? `Meeting ${displayTitle}` : "No active meeting",
    notesMode
      ? `Language ${myName}`
      : `Languages ${myName} and ${meetingName}`,
    capturingNotes ? "Capturing notes" : null,
    ctxLabel ? `Context ${ctxLabel}` : null,
    "Open session menu",
  ]
    .filter(Boolean)
    .join(". ");

  const closeMenu = () => {
    setOpen(false);
    setRenaming(false);
  };

  const handleContextChange = useCallback(
    async (value: string) => {
      const nextId = value === ALWAYS_ON_VALUE ? "" : value;
      setSavingContext(true);
      try {
        await onSave(
          toSavePayload(config, {
            sonioxActiveContextProfileId: nextId,
          }),
        );
      } catch (e) {
        onToast?.("error", String(e));
      } finally {
        setSavingContext(false);
      }
    },
    [config, onSave, onToast],
  );

  const languagesButtonTip = languagesLocked
    ? notesMode
      ? "Language locked while capturing notes. Switch the path to Direct to change language."
      : "Languages locked while translating. Switch a column to Direct to change languages."
    : notesMode
      ? "Change meeting language in Settings"
      : "Change languages in Settings";

  return (
    <div className="relative min-w-0 shrink-0">
      <DropdownMenu
        modal={false}
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (next) setHovering(false);
          if (!next) setRenaming(false);
        }}
      >
        <AppTooltip
          label={tooltipParts.join(" · ")}
          open={!open && hovering}
          contentClassName="max-w-xs"
        >
          <DropdownMenuTrigger asChild>
            <button
              type="button"
              className={cn(
                headerChipTriggerClass(
                  // elapsed | languages | context — equal side cells
                  "inline-flex w-auto items-stretch gap-0 overflow-hidden p-0 font-normal text-foreground",
                ),
                open && "border-border/80 bg-secondary",
                languagesLocked && "opacity-80",
                overBudget && "border-destructive/60",
              )}
              onMouseDown={(e) => e.stopPropagation()}
              onPointerEnter={() => setHovering(true)}
              onPointerLeave={() => setHovering(false)}
              aria-label={ariaLabel}
              aria-expanded={open}
              aria-haspopup="menu"
            >
              {/* Meeting elapsed — left wing only while a live meeting is running */}
              {meeting && meetingElapsed.display ? (
                <>
                  <span
                    className={CHIP_SIDE_CELL_CLASS}
                    aria-label={meetingElapsed.title ?? undefined}
                  >
                    <span
                      role="timer"
                      className="text-xs font-semibold tabular-nums text-muted-foreground"
                    >
                      {meetingElapsed.display}
                    </span>
                  </span>
                  <span
                    className="w-px shrink-0 self-stretch bg-border/80"
                    aria-hidden
                  />
                </>
              ) : null}

              {/* Languages — center wing */}
              <span className="flex min-w-[4.75rem] flex-1 items-center justify-center gap-1.5 px-2.5">
                {notesMode ? (
                  <>
                    {my ? (
                      <FlagIcon
                        languageCode={my.code}
                        countryCode={my.countryCode}
                        className="size-3.5 rounded-[2px]"
                      />
                    ) : null}
                    <span className="text-xs font-medium tabular-nums text-muted-foreground">
                      {myLanguage.toUpperCase()}
                    </span>
                  </>
                ) : (
                  <>
                    {my ? (
                      <FlagIcon
                        languageCode={my.code}
                        countryCode={my.countryCode}
                        className="size-3.5 rounded-[2px]"
                      />
                    ) : null}
                    <ArrowLeftRight
                      size={12}
                      strokeWidth={2}
                      aria-hidden
                      className="text-muted-foreground"
                    />
                    {meetingLang ? (
                      <FlagIcon
                        languageCode={meetingLang.code}
                        countryCode={meetingLang.countryCode}
                        className="size-3.5 rounded-[2px]"
                      />
                    ) : null}
                  </>
                )}
              </span>

              {/* Context — right wing (Soniox only; no empty spacer when idle) */}
              {showSonioxContext ? (
                <>
                  <span
                    className="w-px shrink-0 self-stretch bg-border/80"
                    aria-hidden
                  />
                  <span className={CHIP_SIDE_CELL_CLASS} aria-hidden>
                    <Sparkles
                      strokeWidth={2}
                      className={cn(
                        CHIP_SIDE_ICON_CLASS,
                        "text-muted-foreground",
                        overBudget && "text-destructive",
                      )}
                    />
                  </span>
                </>
              ) : null}
            </button>
          </DropdownMenuTrigger>
        </AppTooltip>

        <DropdownMenuContent
          align="center"
          side="bottom"
          className="w-[min(20rem,calc(100vw-2rem))] p-0"
          onCloseAutoFocus={(e) => e.preventDefault()}
          onInteractOutside={(e) => {
            const target = e.target as HTMLElement | null;
            if (
              target?.closest(
                '[data-slot="select-content"], [data-slot="select-trigger"], [role="listbox"]',
              )
            ) {
              e.preventDefault();
            }
          }}
        >
          <div
            className="flex flex-col gap-3 p-3"
            onPointerDown={(e) => e.stopPropagation()}
          >
            {meeting ? (
              <section className="flex flex-col gap-2">
                <p className="m-0 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                  Meeting
                </p>
                {renaming ? (
                  <InlineTextForm
                    value={meeting.title}
                    placeholder="Meeting title"
                    maxLength={120}
                    onSubmit={async (title) => {
                      if (title !== meeting.title) await onRename(title);
                      closeMenu();
                    }}
                    onCancel={() => setRenaming(false)}
                  />
                ) : (
                  <div className="flex flex-col gap-1.5">
                    <button
                      type="button"
                      className="cursor-pointer rounded-md border border-border bg-card px-2.5 py-1.5 text-left text-sm text-foreground hover:bg-hover-surface"
                      onClick={() => setRenaming(true)}
                    >
                      <span className="block truncate font-medium">
                        {meeting.title}
                      </span>
                      <span className="text-[11px] text-muted-foreground">
                        Click to rename
                      </span>
                    </button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-8 justify-start px-2 text-destructive hover:bg-destructive/10 hover:text-destructive"
                      onClick={() => {
                        closeMenu();
                        setEndConfirmOpen(true);
                      }}
                    >
                      End meeting
                    </Button>
                  </div>
                )}
              </section>
            ) : null}

            {meeting ? <Separator /> : null}

            <section className="flex flex-col gap-2">
              <p className="m-0 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                Session
              </p>
              <div
                className="grid grid-cols-2 gap-1 rounded-md border border-border p-1"
                role="group"
                aria-label="Session mode"
              >
                <Button
                  type="button"
                  variant={notesMode ? "ghost" : "secondary"}
                  size="sm"
                  className="h-8"
                  disabled={sessionModeLocked || savingSessionMode}
                  aria-pressed={!notesMode}
                  onClick={() => void saveSessionMode("interpreter")}
                >
                  Interpreter
                </Button>
                <Button
                  type="button"
                  variant={notesMode ? "secondary" : "ghost"}
                  size="sm"
                  className="h-8"
                  disabled={
                    sessionModeLocked || savingSessionMode || !notesCapable
                  }
                  aria-pressed={notesMode}
                  title={
                    notesCapable
                      ? undefined
                      : "Switch AI provider to Soniox or OpenAI for Notes mode"
                  }
                  onClick={() => void saveSessionMode("notes")}
                >
                  Notes
                </Button>
              </div>
              <p className="m-0 text-[11px] text-muted-foreground">
                {sessionModeLocked && meeting?.status === "live"
                  ? "End meeting to switch Interpreter / Notes."
                  : notesMode
                    ? "Same language — transcript + natural audio, no translation."
                    : notesCapable
                      ? "Different languages — live translate / TTS."
                      : "Notes requires Soniox or OpenAI. Interpreter works with Gemini / OpenAI / Soniox."}
              </p>
            </section>

            <Separator />

            <section className="flex flex-col gap-2">
              <p className="m-0 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                Languages
              </p>
              <AppTooltip label={languagesButtonTip}>
                <span className="inline-flex w-full">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={languagesLocked}
                    className="h-9 w-full justify-between gap-2 px-2.5 font-normal shadow-none"
                    onClick={() => {
                      closeMenu();
                      onOpenLanguages();
                    }}
                  >
                    <span className="inline-flex items-center gap-1.5">
                      {notesMode ? (
                        <>
                          {my ? (
                            <FlagIcon
                              languageCode={my.code}
                              countryCode={my.countryCode}
                              className="size-3.5 rounded-[2px]"
                            />
                          ) : null}
                          <span className="text-xs font-medium tabular-nums">
                            {myLanguage.toUpperCase()}
                          </span>
                        </>
                      ) : (
                        <>
                          {my ? (
                            <FlagIcon
                              languageCode={my.code}
                              countryCode={my.countryCode}
                              className="size-3.5 rounded-[2px]"
                            />
                          ) : null}
                          <span className="text-xs font-medium tabular-nums">
                            {myLanguage.toUpperCase()}
                          </span>
                          <ArrowLeftRight
                            size={12}
                            strokeWidth={2}
                            aria-hidden
                            className="text-muted-foreground"
                          />
                          {meetingLang ? (
                            <FlagIcon
                              languageCode={meetingLang.code}
                              countryCode={meetingLang.countryCode}
                              className="size-3.5 rounded-[2px]"
                            />
                          ) : null}
                          <span className="text-xs font-medium tabular-nums">
                            {meetingLanguage.toUpperCase()}
                          </span>
                        </>
                      )}
                    </span>
                    <ChevronRight
                      size={14}
                      aria-hidden
                      className="text-muted-foreground"
                    />
                  </Button>
                </span>
              </AppTooltip>
            </section>

            {showSonioxContext ? (
              <>
                <Separator />
                <section className="flex flex-col gap-2">
                  <Label
                    htmlFor="session-hub-soniox-context"
                    className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase"
                  >
                    Soniox context
                  </Label>
                  <Select
                    value={selectValue}
                    onValueChange={(value) => void handleContextChange(value)}
                    disabled={contextDisabled || savingContext}
                  >
                    <SelectTrigger
                      id="session-hub-soniox-context"
                      size="sm"
                      className={cn(
                        "w-full shadow-none",
                        overBudget && "border-destructive text-destructive",
                      )}
                    >
                      <SelectValue placeholder="Always-on only" />
                    </SelectTrigger>
                    <SelectContent position="popper">
                      <SelectItem value={ALWAYS_ON_VALUE}>
                        Always-on only
                      </SelectItem>
                      {profiles.map((profile) => (
                        <SelectItem key={profile.id} value={profile.id}>
                          {profile.name}
                          {profile.includeAlwaysOn ? " (+ Always-on)" : ""}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {overBudget ? (
                    <p className="m-0 text-[11px] text-destructive">
                      Context over budget — fix in Settings before Start.
                    </p>
                  ) : null}
                </section>
              </>
            ) : null}
          </div>
        </DropdownMenuContent>
      </DropdownMenu>

      <ConfirmDialog
        open={endConfirmOpen}
        onOpenChange={setEndConfirmOpen}
        title="End this meeting?"
        description="Transcript will be saved."
        confirmLabel="End meeting"
        confirmingLabel="Ending…"
        destructive
        onConfirm={async () => {
          await onEnd();
          setEndConfirmOpen(false);
        }}
      />
    </div>
  );
}
