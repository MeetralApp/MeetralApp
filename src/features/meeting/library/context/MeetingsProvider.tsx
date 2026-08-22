import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import {
  createMeeting,
  createMeetingFolder,
  deleteMeeting,
  deleteMeetingFolder,
  endMeeting,
  listMeetingFolders,
  listMeetings,
  moveMeeting,
  renameMeeting,
  renameMeetingFolder,
  reorderMeetingFolders,
} from "../lib/meetingApi";
import type {
  MeetingFolder,
  MeetingRecord,
} from "../lib/meetingTypes";
import { UNCATEGORIZED_FOLDER_ID } from "../lib/meetingTypes";
import { useMeetingChanged } from "../hooks/useMeetingChanged";
import { MeetingsContext } from "./meetingsContext";

export function MeetingsProvider({ children }: { children: ReactNode }) {
  const [folders, setFolders] = useState<MeetingFolder[]>([]);
  const [meetings, setMeetings] = useState<MeetingRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refreshFolders = useCallback(async () => {
    const list = await listMeetingFolders();
    setFolders(list);
    return list;
  }, []);

  const refreshMeetings = useCallback(async () => {
    const res = await listMeetings(null, 500, 0);
    setMeetings(res.meetings);
    return res.meetings;
  }, []);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await refreshFolders();
      await refreshMeetings();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [refreshFolders, refreshMeetings]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useMeetingChanged(refresh);

  const addFolder = useCallback(async (name: string) => {
    await createMeetingFolder(name);
    await refreshFolders();
  }, [refreshFolders]);

  const renameFolder = useCallback(async (id: string, name: string) => {
    await renameMeetingFolder(id, name);
    await refreshFolders();
  }, [refreshFolders]);

  const removeFolder = useCallback(async (id: string) => {
    await deleteMeetingFolder(id);
    await refresh();
  }, [refresh]);

  const reorderFolders = useCallback(async (folderIds: string[]) => {
    setFolders((prev) => {
      const byId = new Map(prev.map((folder) => [folder.id, folder]));
      return folderIds.flatMap((id, sortOrder) => {
        const folder = byId.get(id);
        return folder ? [{ ...folder, sortOrder }] : [];
      });
    });
    try {
      const next = await reorderMeetingFolders(folderIds);
      setFolders(next);
    } catch (e) {
      await refreshFolders();
      throw e;
    }
  }, [refreshFolders]);

  const addMeeting = useCallback(async (folderId: string | null) => {
    const meeting = await createMeeting(
      undefined,
      folderId === UNCATEGORIZED_FOLDER_ID ? null : folderId,
    );
    await refresh();
    return meeting;
  }, [refresh]);

  const renameMeetingTitle = useCallback(async (id: string, title: string) => {
    await renameMeeting(id, title);
    setMeetings((prev) =>
      prev.map((m) => (m.id === id ? { ...m, title } : m)),
    );
  }, []);

  const moveMeetingToFolder = useCallback(async (id: string, folderId: string | null) => {
    await moveMeeting(
      id,
      folderId === UNCATEGORIZED_FOLDER_ID ? null : folderId,
    );
    await refreshMeetings();
  }, [refreshMeetings]);

  const removeMeeting = useCallback(async (id: string) => {
    await deleteMeeting(id);
    await refresh();
  }, [refresh]);

  const endActiveMeeting = useCallback(async (id: string) => {
    await endMeeting(id);
    await refresh();
  }, [refresh]);

  const value = useMemo(
    () => ({
      folders,
      meetings,
      loading,
      error,
      refresh,
      refreshFolders,
      addFolder,
      renameFolder,
      removeFolder,
      reorderFolders,
      addMeeting,
      renameMeetingTitle,
      moveMeetingToFolder,
      removeMeeting,
      endActiveMeeting,
    }),
    [
      folders,
      meetings,
      loading,
      error,
      refresh,
      refreshFolders,
      addFolder,
      renameFolder,
      removeFolder,
      reorderFolders,
      addMeeting,
      renameMeetingTitle,
      moveMeetingToFolder,
      removeMeeting,
      endActiveMeeting,
    ],
  );

  return (
    <MeetingsContext.Provider value={value}>
      {children}
    </MeetingsContext.Provider>
  );
}
