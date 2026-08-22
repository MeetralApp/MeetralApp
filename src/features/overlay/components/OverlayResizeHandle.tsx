import { useEffect, useRef, type PointerEvent as ReactPointerEvent } from "react";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { overlaySetSize } from "../lib/overlayApi";

export const OVERLAY_MIN_WIDTH = 320;
export const OVERLAY_MIN_HEIGHT = 200;
export const OVERLAY_MAX_WIDTH = 900;
export const OVERLAY_MAX_HEIGHT = 700;

function clamp(n: number, min: number, max: number) {
  return Math.min(max, Math.max(min, n));
}

type Props = {
  width: number;
  height: number;
  onSizeChange?: (width: number, height: number) => void;
};

/** Bottom-right resize grip — only mount when click-through is off. */
export default function OverlayResizeHandle({
  width,
  height,
  onSizeChange,
}: Props) {
  const startRef = useRef<{
    x: number;
    y: number;
    w: number;
    h: number;
  } | null>(null);
  const sizeRef = useRef({ w: width, h: height });

  useEffect(() => {
    sizeRef.current = { w: width, h: height };
  }, [width, height]);

  const onPointerDown = (e: ReactPointerEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    e.preventDefault();
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    startRef.current = {
      x: e.clientX,
      y: e.clientY,
      w: sizeRef.current.w,
      h: sizeRef.current.h,
    };
  };

  const onPointerMove = (e: ReactPointerEvent<HTMLButtonElement>) => {
    const start = startRef.current;
    if (!start) return;
    const nextW = clamp(
      start.w + (e.clientX - start.x),
      OVERLAY_MIN_WIDTH,
      OVERLAY_MAX_WIDTH,
    );
    const nextH = clamp(
      start.h + (e.clientY - start.y),
      OVERLAY_MIN_HEIGHT,
      OVERLAY_MAX_HEIGHT,
    );
    sizeRef.current = { w: nextW, h: nextH };
    onSizeChange?.(nextW, nextH);
    void getCurrentWindow().setSize(new LogicalSize(nextW, nextH));
  };

  const endDrag = (e: ReactPointerEvent<HTMLButtonElement>) => {
    if (!startRef.current) return;
    startRef.current = null;
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
    /* already released */
    }
    const { w, h } = sizeRef.current;
    onSizeChange?.(w, h);
    void overlaySetSize(w, h);
  };

  return (
    <button
      type="button"
      aria-label="Resize overlay"
      data-tauri-drag-region="false"
      className="absolute bottom-0 right-0 z-10 flex h-4 w-4 cursor-se-resize items-end justify-end p-0.5 text-muted-foreground/50 hover:text-muted-foreground"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
    >
      <svg
        width="10"
        height="10"
        viewBox="0 0 10 10"
        aria-hidden
        className="opacity-80"
      >
        <path
          d="M9 1L1 9M9 5L5 9M9 9H9"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.25"
          strokeLinecap="round"
        />
      </svg>
    </button>
  );
}
