import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { ScrollArea } from "@/shared/ui/scroll-area";

import { GuideListMacos, GuideListWindows } from "./SetupGuide";

interface Props {
  open: boolean;
  onClose: () => void;
  platform?: string;
}

export default function SetupGuideModal({
  open,
  onClose,
  platform = "windows",
}: Props) {
  const Guide = platform === "macos" ? GuideListMacos : GuideListWindows;

  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose();
      }}
    >
      <DialogContent className="max-h-[85vh] gap-0 p-0 sm:max-w-lg">
        <DialogHeader className="border-b border-border px-4 py-3">
          <DialogTitle id="setup-guide-modal-title">
            How to set up audio
          </DialogTitle>
        </DialogHeader>
        <ScrollArea className="max-h-[calc(85vh-3.5rem)] px-4 py-3">
          <Guide compact />
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
