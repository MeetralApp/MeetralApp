import { useState, type ReactElement, type ReactNode } from "react";
import { Check, MoreVertical } from "lucide-react";

import InlineTextForm from "@/features/pipeline/components/InlineTextForm";
import ConfirmDialog from "@/shared/components/ConfirmDialog";
import AppTooltip from "@/shared/components/AppTooltip";
import { Button } from "@/shared/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { cn } from "@/shared/lib/utils";
import type {
  MeetingFolder,
  MeetingRecord,
} from "@/features/meeting/library/lib/meetingTypes";
import { UNCATEGORIZED_FOLDER_ID } from "@/features/meeting/library/lib/meetingTypes";

type PanelMode = "idle" | "rename" | "move";

interface Props {
  meeting: MeetingRecord;
  folders: MeetingFolder[];
  onRename: (title: string) => Promise<void>;
  onMove: (folderId: string) => Promise<void>;
  onDelete: () => Promise<void>;
  onRefreshFolders?: () => void | Promise<unknown>;
  /**
  * Custom menu trigger (e.g. header chip button). Defaults to ⋯ icon.
  * Must accept ref + props (`asChild`).
  */
  trigger?: ReactElement;
  /** Tooltip while menu closed (Live hub pattern). */
  tooltipLabel?: ReactNode;
  contentAlign?: "start" | "center" | "end";
  onOpenChange?: (open: boolean) => void;
}

export default function MeetingDetailActions({
  meeting,
  folders,
  onRename,
  onMove,
  onDelete,
  onRefreshFolders,
  trigger,
  tooltipLabel,
  contentAlign,
  onOpenChange,
}: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [hovering, setHovering] = useState(false);
  const [mode, setMode] = useState<PanelMode>("idle");
  const [deleteOpen, setDeleteOpen] = useState(false);
  const isLive = meeting.status === "live";

  const moveOptions = [
    { id: UNCATEGORIZED_FOLDER_ID, name: "Uncategorized" },
    ...folders.map((f) => ({ id: f.id, name: f.name })),
  ];

  const closeMenu = () => {
    setMenuOpen(false);
    setMode("idle");
  };

  if (isLive) {
    return null;
  }

  const handleDelete = async () => {
    await onDelete();
    setDeleteOpen(false);
  };

  const align = contentAlign ?? (trigger ? "center" : "end");

  const defaultTrigger = (
    <Button
      type="button"
      variant="outline"
      size="icon-sm"
      aria-label="Meeting actions"
    >
      <MoreVertical size={18} aria-hidden strokeWidth={2} />
    </Button>
  );

  const menuTrigger = (
    <DropdownMenuTrigger asChild>
      {trigger ?? defaultTrigger}
    </DropdownMenuTrigger>
  );

  return (
    <div className="w-full min-w-0">
      <DropdownMenu
        modal={false}
        open={menuOpen}
        onOpenChange={(open) => {
          setMenuOpen(open);
          if (!open) setMode("idle");
          if (open) setHovering(false);
          onOpenChange?.(open);
        }}
      >
        {tooltipLabel ? (
          <AppTooltip
            label={tooltipLabel}
            open={!menuOpen && hovering}
            contentClassName="max-w-xs"
          >
            <span
              className="flex w-full min-w-0"
              onPointerEnter={() => setHovering(true)}
              onPointerLeave={() => setHovering(false)}
            >
              {menuTrigger}
            </span>
          </AppTooltip>
        ) : (
          menuTrigger
        )}
        <DropdownMenuContent
          align={align}
          side="bottom"
          className={cn(
            "min-w-[220px]",
            mode === "rename" && "w-[min(100vw-2rem,22rem)]",
            mode === "move" &&
              "max-w-[min(100vw-2rem,320px)] overflow-hidden",
          )}
          onCloseAutoFocus={(e) => e.preventDefault()}
        >
          {mode === "idle" && (
            <>
              <DropdownMenuItem
                onSelect={(e) => {
                  e.preventDefault();
                  setMode("rename");
                }}
              >
                Rename
              </DropdownMenuItem>
              <DropdownMenuItem
                onSelect={(e) => {
                  e.preventDefault();
                  void onRefreshFolders?.();
                  setMode("move");
                }}
              >
                Move to folder
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                variant="destructive"
                onSelect={(e) => {
                  e.preventDefault();
                  closeMenu();
                  setDeleteOpen(true);
                }}
              >
                Delete
              </DropdownMenuItem>
            </>
          )}
          {mode === "rename" && (
            <div className="p-1">
              <InlineTextForm
                value={meeting.title}
                placeholder="Meeting title"
                maxLength={120}
                onSubmit={async (title) => {
                  if (title !== meeting.title) await onRename(title);
                  closeMenu();
                }}
                onCancel={() => setMode("idle")}
              />
            </div>
          )}
          {mode === "move" && (
            <div className="flex min-w-0 flex-col">
              <DropdownMenuLabel className="shrink-0 text-xs font-semibold text-muted-foreground">
                Move to folder
              </DropdownMenuLabel>
              <div
                className="max-h-[200px] min-h-0 overflow-x-hidden overflow-y-auto overscroll-contain pr-0.5"
                role="listbox"
                aria-label="Folders"
              >
                {moveOptions.map((opt) => {
                  const isCurrent =
                    meeting.folderId === opt.id ||
                    (!meeting.folderId && opt.id === UNCATEGORIZED_FOLDER_ID);
                  return (
                    <DropdownMenuItem
                      key={opt.id}
                      className={cn(
                        "min-w-0",
                        isCurrent &&
                          "bg-primary/10 text-primary focus:bg-primary/10 focus:text-primary",
                      )}
                      onSelect={(e) => {
                        e.preventDefault();
                        void (async () => {
                          if (!isCurrent) await onMove(opt.id);
                          closeMenu();
                        })();
                      }}
                    >
                      <Check
                        className={cn(
                          "size-3.5 shrink-0",
                          !isCurrent && "invisible",
                        )}
                        aria-hidden
                      />
                      <span className="min-w-0 flex-1 truncate">{opt.name}</span>
                    </DropdownMenuItem>
                  );
                })}
              </div>
              <DropdownMenuSeparator className="shrink-0" />
              <DropdownMenuItem
                className="shrink-0"
                onSelect={(e) => {
                  e.preventDefault();
                  setMode("idle");
                }}
              >
                Cancel
              </DropdownMenuItem>
            </div>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      <ConfirmDialog
        open={deleteOpen}
        onOpenChange={setDeleteOpen}
        title="Delete meeting?"
        description={
          <>
            <span className="block break-all font-medium text-foreground">
              “{meeting.title}”
            </span>
            <span className="mt-1.5 block">This cannot be undone.</span>
          </>
        }
        confirmLabel="Delete"
        destructive
        onConfirm={() => void handleDelete()}
      />
    </div>
  );
}
