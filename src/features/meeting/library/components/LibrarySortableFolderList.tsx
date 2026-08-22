import type { ReactNode } from "react";
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";

import LibraryFolderBlock from "./LibraryFolderBlock";
import type { MeetingFolder, MeetingRecord } from "../lib/meetingTypes";
import AppTooltip from "@/shared/components/AppTooltip";
import { cn } from "@/shared/lib/utils";

interface SortableFolderRowProps {
  folder: MeetingFolder;
  sortDisabled: boolean;
  expanded: boolean;
  selected: boolean;
  editing: boolean;
  count: number;
  childMeetings: MeetingRecord[];
  activeMeetingId: string | null;
  onExpand: () => void;
  onSelect: () => void;
  onSelectMeeting: (meeting: MeetingRecord) => void;
  onNameSubmit: (name: string) => void | Promise<void>;
  onNameCancel: () => void;
}

function SortableFolderRow({
  folder,
  sortDisabled,
  ...blockProps
}: SortableFolderRowProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: folder.id,
    disabled: sortDisabled,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  const dragHandle = sortDisabled ? null : (
    <AppTooltip label="Drag to reorder">
      <button
        type="button"
        className="grid h-7 w-5 shrink-0 cursor-grab touch-none place-items-center rounded-md border-none bg-transparent text-muted-foreground hover:bg-hover-surface hover:text-foreground active:cursor-grabbing"
        aria-label={`Reorder folder ${folder.name}`}
        {...attributes}
        {...listeners}
      >
        <GripVertical size={14} aria-hidden strokeWidth={2} />
      </button>
    </AppTooltip>
  );

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        "min-w-0 w-full",
        isDragging && "relative z-10 opacity-80",
      )}
    >
      <LibraryFolderBlock
        folderId={folder.id}
        name={folder.name}
        depth={0}
        dragHandle={dragHandle}
        {...blockProps}
      />
    </div>
  );
}

interface Props {
  folders: MeetingFolder[];
  sortDisabled: boolean;
  expandedFolders: Set<string>;
  selectedFolderId: string | null;
  creatingFolder: boolean;
  folderPanelMode: "idle" | "rename" | "delete";
  activeMeetingId: string | null;
  meetingsByFolder: Map<string | null, MeetingRecord[]>;
  folderCount: (id: string | null) => number;
  onSelectFolder: (folderId: string) => void;
  onSelectMeeting: (meeting: MeetingRecord) => void;
  onRenameFolder: (id: string, name: string) => Promise<void>;
  onFolderPanelModeChange: (mode: "idle" | "rename" | "delete") => void;
  onToggleExpand: (folderId: string) => void;
  onReorderFolders: (folderIds: string[]) => void | Promise<void>;
  draftRow?: ReactNode;
}

export default function LibrarySortableFolderList({
  folders,
  sortDisabled,
  expandedFolders,
  selectedFolderId,
  creatingFolder,
  folderPanelMode,
  activeMeetingId,
  meetingsByFolder,
  folderCount,
  onSelectFolder,
  onSelectMeeting,
  onRenameFolder,
  onFolderPanelModeChange,
  onToggleExpand,
  onReorderFolders,
  draftRow,
}: Props) {
  const folderIds = folders.map((folder) => folder.id);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 6 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id || sortDisabled) return;

    const oldIndex = folderIds.indexOf(String(active.id));
    const newIndex = folderIds.indexOf(String(over.id));
    if (oldIndex < 0 || newIndex < 0) return;

    const next = [...folderIds];
    const [moved] = next.splice(oldIndex, 1);
    next.splice(newIndex, 0, moved);
    void onReorderFolders(next);
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
    >
      <SortableContext items={folderIds} strategy={verticalListSortingStrategy}>
        {draftRow}
        {folders.map((folder) => (
          <SortableFolderRow
            key={folder.id}
            folder={folder}
            sortDisabled={sortDisabled}
            expanded={expandedFolders.has(folder.id)}
            selected={!creatingFolder && selectedFolderId === folder.id}
            editing={
              folderPanelMode === "rename" && selectedFolderId === folder.id
            }
            count={folderCount(folder.id)}
            childMeetings={meetingsByFolder.get(folder.id) ?? []}
            activeMeetingId={activeMeetingId}
            onExpand={() => onToggleExpand(folder.id)}
            onSelect={() => onSelectFolder(folder.id)}
            onSelectMeeting={onSelectMeeting}
            onNameSubmit={async (folderName) => {
              if (folderName !== folder.name) {
                await onRenameFolder(folder.id, folderName);
              }
              onFolderPanelModeChange("idle");
            }}
            onNameCancel={() => onFolderPanelModeChange("idle")}
          />
        ))}
      </SortableContext>
    </DndContext>
  );
}
