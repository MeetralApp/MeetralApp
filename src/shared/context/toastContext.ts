import { createContext } from "react";

import type { ToastType } from "./toastTypes";

export interface ToastContextValue {
  showToast: (type: ToastType, text: string) => void;
}

export const ToastContext = createContext<ToastContextValue | null>(null);
