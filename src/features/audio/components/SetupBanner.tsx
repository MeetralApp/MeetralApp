import LiveNotice from "@/shared/components/LiveNotice";

import type { ConfigView, AppStatus } from "@/shared/lib/types/pipeline";
import {
  buildSetupIssues,
  isSetupBannerVisible,
  type AudioSetupValidation,
} from "../lib/audioSetup";

interface Props {
  config: ConfigView;
  status?: AppStatus | null;
  audioSetup: AudioSetupValidation | null;
  canDirectOutbound: boolean;
  canDirectInbound: boolean;
  canTranslateOutbound: boolean;
  canTranslateInbound: boolean;
  onOpenSettings: () => void;
}

function summarizeSetupIssues(issues: string[]): string {
  if (issues.length === 0) return "Finish audio setup in Settings.";
  if (issues.length === 1) return issues[0];
  const extra = issues.length - 1;
  return `${issues[0]} (+${extra} more)`;
}

export default function SetupBanner({
  config,
  status = null,
  audioSetup,
  canDirectOutbound,
  canDirectInbound,
  canTranslateOutbound,
  canTranslateInbound,
  onOpenSettings,
}: Props) {
  if (
    !isSetupBannerVisible({
      config,
      status,
      audioSetup,
      canDirectOutbound,
      canDirectInbound,
      canTranslateOutbound,
      canTranslateInbound,
    })
  ) {
    return null;
  }

  const message = summarizeSetupIssues(buildSetupIssues(config, audioSetup));

  return (
    <LiveNotice.Strip
      message={message}
      actions={[{ label: "Settings", onClick: onOpenSettings }]}
    />
  );
}
