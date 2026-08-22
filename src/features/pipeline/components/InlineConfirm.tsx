import ConfirmDialog from "@/shared/components/ConfirmDialog";

interface Props {
  title: React.ReactNode;
  description?: React.ReactNode;
  /** @deprecated Use `title` instead */
  message?: string;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function InlineConfirm({
  title,
  description,
  message,
  confirmLabel = "Confirm",
  onConfirm,
  onCancel,
}: Props) {
  return (
    <ConfirmDialog
      open
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
      title={title ?? message ?? ""}
      description={description}
      confirmLabel={confirmLabel}
      destructive
      onConfirm={onConfirm}
    />
  );
}
