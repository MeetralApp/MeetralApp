export type ToastType = "success" | "info" | "error" | "warning";

export const TOAST_DURATION: Record<ToastType, number> = {
  success: 3000,
  info: 4000,
  warning: 4500,
  error: 6000,
};
