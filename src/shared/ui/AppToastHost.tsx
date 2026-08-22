import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import {
  CircleAlertIcon,
  CircleCheckIcon,
  InfoIcon,
  TriangleAlertIcon,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import type { ToastType } from "@/shared/context/toastTypes";

export type AppToastItem = {
  id: number;
  type: ToastType;
  text: string;
};

const ICONS: Record<ToastType, typeof CircleCheckIcon> = {
  success: CircleCheckIcon,
  info: InfoIcon,
  warning: TriangleAlertIcon,
  error: CircleAlertIcon,
};

const ICON_CLASS: Record<ToastType, string> = {
  success: "text-success",
  info: "text-accent",
  warning: "text-warning",
  error: "text-destructive",
};

type Props = {
  toast: AppToastItem | null;
};

/**
* First-party toast host for Tauri WebView.
* Portal to document.body; chip is in normal flow inside a fixed bottom-center
* region (no nested position:fixed / Sonner transforms).
*/
export function AppToastHost({ toast }: Props) {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted || !toast || typeof document === "undefined") {
    return null;
  }

  const Icon = ICONS[toast.type];

  return createPortal(
    <div
      data-app-toast-host=""
      className="pointer-events-none fixed inset-x-0 bottom-[18px] z-[2147483646] flex justify-center px-4"
      role="region"
      aria-label="Notifications"
    >
      <div
        key={toast.id}
        data-app-toast=""
        data-type={toast.type}
        role="status"
        aria-live="polite"
        className={cn(
          "app-toast-chip pointer-events-auto flex max-w-[min(360px,calc(100vw-32px))] items-center gap-2",
          "rounded-[var(--radius)] border border-border bg-card px-3 py-2",
          "text-sm font-medium leading-snug text-foreground",
          "shadow-[0_2px_10px_rgba(0,0,0,0.08)] dark:shadow-[0_2px_8px_rgba(0,0,0,0.35)]",
        )}
      >
        <Icon
          className={cn("size-4 shrink-0", ICON_CLASS[toast.type])}
          aria-hidden
        />
        <span className="min-w-0">{toast.text}</span>
      </div>
    </div>,
    document.body,
  );
}
