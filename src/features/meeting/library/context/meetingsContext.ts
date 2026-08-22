import { createContext } from "react";

import type {
  MeetingFolder,
  MeetingRecord,
} from "../lib/meetingTypes";

export interface MeetingsContextValue {
  folders: MeetingFolder[];
  meetings: MeetingRecord[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  refreshFolders: () => Promise<MeetingFolder[]>;
  addFolder: (name: string) => Promise<void>;
  renameFolder: (id: string, name: string) => Promise<void>;
  removeFolder: (id: string) => Promise<void>;
  reorderFolders: (folderIds: string[]) => Promise<void>;
  addMeeting: (folderId: string | null) => Promise<MeetingRecord>;
  renameMeetingTitle: (id: string, title: string) => Promise<void>;
  moveMeetingToFolder: (id: string, folderId: string | null) => Promise<void>;
  removeMeeting: (id: string) => Promise<void>;
  endActiveMeeting: (id: string) => Promise<void>;
}

export const MeetingsContext = createContext<MeetingsContextValue | null>(null);
