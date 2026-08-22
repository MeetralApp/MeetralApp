import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

import { AppToastHost, type AppToastItem } from "@/shared/ui/AppToastHost";
import { ToastContext } from "./toastContext";
import { TOAST_DURATION, type ToastType } from "./toastTypes";

let nextToastId = 1;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toast, setToast] = useState<AppToastItem | null>(null);
  const dismissTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearDismissTimer = useCallback(() => {
    if (dismissTimerRef.current != null) {
      clearTimeout(dismissTimerRef.current);
      dismissTimerRef.current = null;
    }
  }, []);

  const showToast = useCallback(
    (type: ToastType, text: string) => {
      clearDismissTimer();
      const id = nextToastId++;
      setToast({ id, type, text });
      dismissTimerRef.current = setTimeout(() => {
        setToast((current) => (current?.id === id ? null : current));
        dismissTimerRef.current = null;
      }, TOAST_DURATION[type]);
    },
    [clearDismissTimer],
  );

  useEffect(() => {
    return () => clearDismissTimer();
  }, [clearDismissTimer]);

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      <AppToastHost toast={toast} />
    </ToastContext.Provider>
  );
}
