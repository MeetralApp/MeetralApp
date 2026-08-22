import LiveNotice, {
  type LiveNoticeActionSpec,
} from "@/shared/components/LiveNotice";

import {
  AUDIO_ROLE_UNAVAILABLE_TITLES,
  type PipelineErrorNotice,
} from "../lib/audioSetup";

interface Props {
  notice: PipelineErrorNotice;
  onOpenAudioSettings: () => void;
  onUseSystemDefault?: () => Promise<void>;
}

export default function PipelineErrorBanner({
  notice,
  onOpenAudioSettings,
  onUseSystemDefault,
}: Props) {
  if (notice.kind === "fatal") {
    return <LiveNotice.Strip tone="destructive" message={notice.message} />;
  }

  const title = notice.role
    ? AUDIO_ROLE_UNAVAILABLE_TITLES[notice.role]
    : "Audio device unavailable";
  const detail = notice.deviceName?.trim() || null;
  const actions: LiveNoticeActionSpec[] = [];
  if (notice.allowSystemDefault && onUseSystemDefault) {
    actions.push({
      label: "Use default",
      intent: "secondary",
      onClick: onUseSystemDefault,
    });
  }
  actions.push({ label: "Fix", onClick: onOpenAudioSettings });

  return <LiveNotice.Strip title={title} detail={detail} actions={actions} />;
}
