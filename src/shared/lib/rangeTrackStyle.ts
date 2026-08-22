import type { CSSProperties } from "react";

/** Filled (primary) + unfilled (--slider-track) gradient for native range inputs. */
export function rangeTrackStyle(
  value: number,
  min: number,
  max: number,
): CSSProperties {
  const span = max - min;
  const pct =
    span <= 0 ? 0 : Math.min(100, Math.max(0, ((value - min) / span) * 100));
  return {
    background: `linear-gradient(to right, var(--primary) ${pct}%, var(--slider-track) ${pct}%)`,
  };
}
