import { useCallback, useState } from "react";

import type { AppView } from "@/shared/lib/types/appView";
import type { SettingsFocus } from "@/features/config/lib/settingsFocus";
import type { PendingScrollSegment } from "@/features/meeting/library/lib/meetingTypes";

export function useAppNavigation() {
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [libraryOpen, setLibraryOpen] = useState(false);
  const [settingsFocus, setSettingsFocus] = useState<SettingsFocus | null>(
    null,
  );
  const [appView, setAppView] = useState<AppView>("live");
  const [detailMeetingId, setDetailMeetingId] = useState<string | null>(null);
  const [pendingScrollSegment, setPendingScrollSegment] =
    useState<PendingScrollSegment | null>(null);

  const openSettings = useCallback((focus?: SettingsFocus) => {
    setLibraryOpen(false);
    setSettingsFocus(focus ?? null);
    setDrawerOpen(true);
  }, []);

  const openLibrary = useCallback(() => {
    setDrawerOpen(false);
    setSettingsFocus(null);
    setLibraryOpen((v) => !v);
  }, []);

  const openMeetingDetail = useCallback(
    (meetingId: string, scroll?: PendingScrollSegment | null) => {
      setDetailMeetingId(meetingId);
      setPendingScrollSegment(
        scroll && scroll.meetingId === meetingId ? scroll : null,
      );
      setAppView("meeting-detail");
      setLibraryOpen(false);
    },
    [],
  );

  const consumePendingScrollSegment = useCallback(() => {
    setPendingScrollSegment(null);
  }, []);

  const backToLive = useCallback(() => {
    setAppView("live");
    setDetailMeetingId(null);
    setPendingScrollSegment(null);
  }, []);

  const closeSettings = useCallback(() => {
    setDrawerOpen(false);
    setSettingsFocus(null);
  }, []);

  const closeLibrary = useCallback(() => {
    setLibraryOpen(false);
  }, []);

  return {
    appView,
    detailMeetingId,
    pendingScrollSegment,
    drawerOpen,
    libraryOpen,
    settingsFocus,
    openSettings,
    openLibrary,
    openMeetingDetail,
    consumePendingScrollSegment,
    backToLive,
    closeSettings,
    closeLibrary,
    setLibraryOpen,
  };
}
