import { useCallback, useEffect, useState } from "react";
import { Plus, X } from "lucide-react";

import LibrarySearch from "./LibrarySearch";
import LibraryTree, { type FolderPanelMode } from "./LibraryTree";
import AppTooltip from "@/shared/components/AppTooltip";
import SectionHeading from "@/shared/components/SectionHeading";
import { Button } from "@/shared/ui/button";
import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/shared/ui/sheet";
import { useMeetings } from "../hooks/useMeetings";
import type { NewMeetingLock } from "@/features/pipeline/lib/pipelineStatus";
import { UNCATEGORIZED_FOLDER_ID } from "../lib/meetingTypes";
import type {
  MeetingRecord,
  MeetingSearchHit,
} from "../lib/meetingTypes";

interface Props {
  open: boolean;
  onClose: () => void;
  onSelectMeeting: (id: string) => void;
  onSelectSearchHit: (hit: MeetingSearchHit) => void;
  onNewMeetingStarted: () => void;
  onReturnToLive: () => void;
  onMeetingCreated?: () => void;
  activeMeetingId: string | null;
  newMeetingLock: NewMeetingLock;
}

export default function LibraryDrawer({
  open,
  onClose,
  onSelectMeeting,
  onSelectSearchHit,
  onNewMeetingStarted,
  onReturnToLive,
  onMeetingCreated,
  activeMeetingId,
  newMeetingLock,
}: Props) {
  const [newMeetingFolderId, setNewMeetingFolderId] = useState<string | null>(
    null,
  );
  const [selectedFolderId, setSelectedFolderId] = useState<string | null>(null);
  const [folderPanelMode, setFolderPanelMode] =
    useState<FolderPanelMode>("idle");
  const [searchOpen, setSearchOpen] = useState(false);
  const {
    folders,
    meetings,
    loading,
    error,
    addFolder,
    renameFolder,
    removeFolder,
    reorderFolders,
    addMeeting,
    refresh,
  } = useMeetings();

  useEffect(() => {
    if (!open) {
      setSelectedFolderId(null);
      setFolderPanelMode("idle");
      setSearchOpen(false);
      return;
    }
    void refresh();
  }, [open, refresh]);

  const handleNewMeetingFolderChange = useCallback(
    (folderId: string | null) => {
      setNewMeetingFolderId(folderId);
    },
    [],
  );

  const folderIdForNewMeeting = (() => {
    if (!newMeetingFolderId || newMeetingFolderId === UNCATEGORIZED_FOLDER_ID) {
      return null;
    }
    return newMeetingFolderId;
  })();

  const sessionBlocksNewMeeting = Boolean(activeMeetingId);
  const newMeetingDisabled =
    newMeetingLock.locked || sessionBlocksNewMeeting;

  const handleNewMeeting = async () => {
    if (newMeetingDisabled) return;
    await addMeeting(folderIdForNewMeeting);
    onClose();
    onNewMeetingStarted();
    onMeetingCreated?.();
  };

  const handleSelectMeeting = (meeting: MeetingRecord) => {
    onClose();
    if (meeting.status === "live") {
      onReturnToLive();
    } else {
      onSelectMeeting(meeting.id);
    }
  };

  const handleSelectSearchHit = (hit: MeetingSearchHit) => {
    onClose();
    onSelectSearchHit(hit);
  };

  const handleSelectFolder = (folderId: string | null) => {
    setSelectedFolderId(folderId);
    setFolderPanelMode("idle");
  };

  const newMeetingTitle = newMeetingLock.locked
    ? newMeetingLock.hint
    : sessionBlocksNewMeeting
      ? "End the current meeting before starting a new one"
      : folderIdForNewMeeting
        ? `New meeting in ${folders.find((f) => f.id === folderIdForNewMeeting)?.name ?? "folder"}`
        : "New meeting in Uncategorized";

  return (
    <Sheet open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <SheetContent
        side="left"
        showCloseButton={false}
        className="flex w-[320px] max-w-[92vw] flex-col gap-0 p-0 sm:max-w-[320px]"
        aria-label="Meeting history"
      >
        <SheetHeader className="flex-row items-center justify-between space-y-0 border-b px-3 py-2">
          <SheetTitle asChild>
            <SectionHeading as="h2">History</SectionHeading>
          </SheetTitle>
          <SheetClose asChild>
            <Button variant="ghost" size="icon" aria-label="Close library">
              <X className="size-4" aria-hidden strokeWidth={2} />
            </Button>
          </SheetClose>
        </SheetHeader>

        <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden px-3 pt-1.5">
          <LibrarySearch
            drawerOpen={open}
            searchOpen={searchOpen}
            onSearchOpenChange={setSearchOpen}
            meetings={meetings}
            onSelectHit={handleSelectSearchHit}
            footerAction={
              <AppTooltip label={newMeetingTitle}>
                <span className="inline-flex w-full">
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    className="w-full"
                    disabled={newMeetingDisabled}
                    onClick={() => void handleNewMeeting()}
                  >
                    <Plus size={16} aria-hidden strokeWidth={2} />
                    New meeting
                  </Button>
                </span>
              </AppTooltip>
            }
          >
            <LibraryTree
              folders={folders}
              meetings={meetings}
              activeMeetingId={activeMeetingId}
              selectedFolderId={selectedFolderId}
              folderPanelMode={folderPanelMode}
              loading={loading}
              error={error}
              onSelectMeeting={handleSelectMeeting}
              onSelectFolder={handleSelectFolder}
              onCreateFolder={(name) => void addFolder(name)}
              onNewMeetingFolderChange={handleNewMeetingFolderChange}
              onFolderPanelModeChange={setFolderPanelMode}
              onRenameFolder={async (id, name) => {
                await renameFolder(id, name);
              }}
              onDeleteFolder={async (id) => {
                await removeFolder(id);
                setSelectedFolderId(null);
              }}
              onReorderFolders={async (folderIds) => {
                await reorderFolders(folderIds);
              }}
            />
          </LibrarySearch>
        </div>
      </SheetContent>
    </Sheet>
  );
}
