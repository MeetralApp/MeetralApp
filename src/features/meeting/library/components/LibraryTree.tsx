import { useEffect, useMemo, useState } from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";

import InlineConfirm from "@/features/pipeline/components/InlineConfirm";
import LibraryFolderBlock from "./LibraryFolderBlock";
import LibraryMeetingRow from "./LibraryMeetingRow";
import LibrarySortableFolderList from "./LibrarySortableFolderList";
import AppTooltip from "@/shared/components/AppTooltip";
import SectionHeading from "@/shared/components/SectionHeading";
import { Button } from "@/shared/ui/button";
import type { MeetingFolder, MeetingRecord } from "../lib/meetingTypes";
import { UNCATEGORIZED_FOLDER_ID } from "../lib/meetingTypes";

const RECENT_LIMIT = 3;
const EXPANDED_STORAGE_KEY = "library-tree-expanded-folders";
const NEW_FOLDER_DRAFT_ID = "__new_folder__";

export type FolderPanelMode = "idle" | "rename" | "delete";

interface Props {
  folders: MeetingFolder[];
  meetings: MeetingRecord[];
  activeMeetingId: string | null;
  selectedFolderId: string | null;
  folderPanelMode: FolderPanelMode;
  loading: boolean;
  error: string | null;
  onSelectMeeting: (meeting: MeetingRecord) => void;
  onSelectFolder: (folderId: string | null) => void;
  onCreateFolder: (name: string) => void;
  onNewMeetingFolderChange: (folderId: string | null) => void;
  onFolderPanelModeChange: (mode: FolderPanelMode) => void;
  onRenameFolder: (id: string, name: string) => Promise<void>;
  onDeleteFolder: (id: string) => Promise<void>;
  onReorderFolders: (folderIds: string[]) => void | Promise<void>;
}

function loadExpandedFolders(): Set<string> {
  try {
    const raw = localStorage.getItem(EXPANDED_STORAGE_KEY);
    if (!raw) return new Set();
    const parsed = JSON.parse(raw) as string[];
    return new Set(parsed);
  } catch {
    return new Set();
  }
}

function saveExpandedFolders(set: Set<string>) {
  localStorage.setItem(EXPANDED_STORAGE_KEY, JSON.stringify([...set]));
}

export default function LibraryTree({
  folders,
  meetings,
  activeMeetingId,
  selectedFolderId,
  folderPanelMode,
  loading,
  error,
  onSelectMeeting,
  onSelectFolder,
  onCreateFolder,
  onNewMeetingFolderChange,
  onFolderPanelModeChange,
  onRenameFolder,
  onDeleteFolder,
  onReorderFolders,
}: Props) {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(
    loadExpandedFolders,
  );
  const [creatingFolder, setCreatingFolder] = useState(false);
  const [folderSelectionBeforeCreate, setFolderSelectionBeforeCreate] =
    useState<string | null>(null);

  useEffect(() => {
    saveExpandedFolders(expandedFolders);
  }, [expandedFolders]);

  useEffect(() => {
    onNewMeetingFolderChange(selectedFolderId);
  }, [selectedFolderId, onNewMeetingFolderChange]);

  const liveMeeting = useMemo(
    () => meetings.find((m) => m.status === "live") ?? null,
    [meetings],
  );

  const recentMeetings = useMemo(() => {
    return meetings
      .filter((m) => m.status !== "live")
      .sort((a, b) => b.startedAtMs - a.startedAtMs)
      .slice(0, RECENT_LIMIT);
  }, [meetings]);

  const meetingsByFolder = useMemo(() => {
    const map = new Map<string | null, MeetingRecord[]>();
    for (const m of meetings) {
      const key = m.folderId;
      const list = map.get(key) ?? [];
      list.push(m);
      map.set(key, list);
    }
    for (const [, list] of map) {
      list.sort((a, b) => b.startedAtMs - a.startedAtMs);
    }
    return map;
  }, [meetings]);

  const uncategorizedMeetings = meetingsByFolder.get(null) ?? [];

  const selectedRealFolder =
    selectedFolderId && selectedFolderId !== UNCATEGORIZED_FOLDER_ID
      ? folders.find((f) => f.id === selectedFolderId) ?? null
      : null;

  const toggleExpand = (storageKey: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(storageKey)) next.delete(storageKey);
      else next.add(storageKey);
      return next;
    });
  };

  const folderCount = (id: string | null) =>
    meetingsByFolder.get(id)?.length ?? 0;

  const startCreatingFolder = () => {
    setFolderSelectionBeforeCreate(selectedFolderId);
    onFolderPanelModeChange("idle");
    onSelectFolder(null);
    setCreatingFolder(true);
  };

  const finishCreatingFolder = (restoreSelection: boolean) => {
    setCreatingFolder(false);
    if (restoreSelection && folderSelectionBeforeCreate) {
      onSelectFolder(folderSelectionBeforeCreate);
    }
    setFolderSelectionBeforeCreate(null);
  };

  const headerActionsVisible =
    folderPanelMode === "idle" && !creatingFolder;

  const folderSortDisabled =
    creatingFolder || folderPanelMode !== "idle";

  return (
    <div
      className="flex min-h-0 min-w-0 flex-1 flex-col gap-0"
      role="tree"
      aria-label="Meeting library"
    >
      {loading && (
        <p className="m-0 px-2 py-1 text-sm text-muted-foreground">Loading…</p>
      )}
      {error && (
        <p className="m-0 px-2 py-1 text-sm text-destructive">{error}</p>
      )}

      {!loading && liveMeeting && (
        <div className="mb-1.5 shrink-0 border-b border-border pb-2">
          <div className="mb-2 flex items-center justify-between gap-2">
            <SectionHeading>Current session</SectionHeading>
          </div>
          <LibraryMeetingRow
            meeting={liveMeeting}
            depth={0}
            pinned
            onSelect={() => onSelectMeeting(liveMeeting)}
          />
        </div>
      )}

      {!loading && (
        <div className="mb-1.5 min-w-0 shrink-0 border-b border-border pb-2">
          <div className="mb-2 flex items-center justify-between gap-2">
            <SectionHeading>Recent</SectionHeading>
          </div>
          {recentMeetings.map((m) => (
            <LibraryMeetingRow
              key={`recent-${m.id}`}
              meeting={m}
              depth={0}
              onSelect={() => onSelectMeeting(m)}
            />
          ))}
          {recentMeetings.length === 0 && (
            <p className="m-0 px-2 py-1 text-sm text-muted-foreground">
              No recent meetings
            </p>
          )}
        </div>
      )}

      {!loading && (
        <div className="flex min-h-0 min-w-0 flex-1 flex-col pb-1">
          <div className="mb-2 flex items-center justify-between gap-2">
            <SectionHeading>Folders</SectionHeading>
            {headerActionsVisible && (
              <div className="flex shrink-0 items-center gap-0.5">
                {selectedRealFolder && (
                  <>
                    <AppTooltip label="Rename folder">
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-xs"
                        aria-label={`Rename folder ${selectedRealFolder.name}`}
                        onClick={() => onFolderPanelModeChange("rename")}
                      >
                        <Pencil size={14} aria-hidden strokeWidth={2} />
                      </Button>
                    </AppTooltip>
                    <AppTooltip label="Delete folder">
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-xs"
                        className="text-destructive hover:text-destructive"
                        aria-label={`Delete folder ${selectedRealFolder.name}`}
                        onClick={() => onFolderPanelModeChange("delete")}
                      >
                        <Trash2 size={14} aria-hidden strokeWidth={2} />
                      </Button>
                    </AppTooltip>
                  </>
                )}
                <AppTooltip label="New folder">
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-xs"
                    aria-label="New folder"
                    onClick={startCreatingFolder}
                  >
                    <Plus size={14} aria-hidden strokeWidth={2} />
                  </Button>
                </AppTooltip>
              </div>
            )}
          </div>

          {selectedRealFolder && folderPanelMode === "delete" && (
            <InlineConfirm
              title="Delete folder?"
              description={
                <>
                  <span className="block break-all font-medium text-foreground">
                    “{selectedRealFolder.name}”
                  </span>
                  <span className="mt-1.5 block">
                    Meetings move to Uncategorized.
                  </span>
                </>
              }
              confirmLabel="Delete"
              onConfirm={async () => {
                await onDeleteFolder(selectedRealFolder.id);
                onFolderPanelModeChange("idle");
              }}
              onCancel={() => onFolderPanelModeChange("idle")}
            />
          )}

          <div className="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto pr-1">
            <div role="group" className="min-w-0 w-full pr-1.5">
              <LibrarySortableFolderList
                folders={folders}
                sortDisabled={folderSortDisabled}
                expandedFolders={expandedFolders}
                selectedFolderId={selectedFolderId}
                creatingFolder={creatingFolder}
                folderPanelMode={folderPanelMode}
                activeMeetingId={activeMeetingId}
                meetingsByFolder={meetingsByFolder}
                folderCount={folderCount}
                onSelectFolder={onSelectFolder}
                onSelectMeeting={onSelectMeeting}
                onRenameFolder={onRenameFolder}
                onFolderPanelModeChange={onFolderPanelModeChange}
                onToggleExpand={toggleExpand}
                onReorderFolders={onReorderFolders}
                draftRow={
                  creatingFolder ? (
                    <LibraryFolderBlock
                      draft
                      folderId={NEW_FOLDER_DRAFT_ID}
                      name=""
                      expanded={false}
                      selected={false}
                      count={0}
                      childMeetings={[]}
                      activeMeetingId={activeMeetingId}
                      depth={0}
                      editing
                      onExpand={() => {}}
                      onSelect={() => {}}
                      onSelectMeeting={onSelectMeeting}
                      onNameSubmit={(folderName) => {
                        onCreateFolder(folderName);
                        finishCreatingFolder(false);
                      }}
                      onNameCancel={() => finishCreatingFolder(true)}
                    />
                  ) : null
                }
              />
              <LibraryFolderBlock
                folderId={UNCATEGORIZED_FOLDER_ID}
                name="Uncategorized"
                expanded={expandedFolders.has(UNCATEGORIZED_FOLDER_ID)}
                selected={
                  !creatingFolder && selectedFolderId === UNCATEGORIZED_FOLDER_ID
                }
                count={folderCount(null)}
                childMeetings={uncategorizedMeetings}
                activeMeetingId={activeMeetingId}
                depth={0}
                onExpand={() => toggleExpand(UNCATEGORIZED_FOLDER_ID)}
                onSelect={() => onSelectFolder(UNCATEGORIZED_FOLDER_ID)}
                onSelectMeeting={onSelectMeeting}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
