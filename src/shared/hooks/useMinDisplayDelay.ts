import { useEffect, useRef, useState } from "react";

/**
* Keeps `active` true for at least `minMs` after it first became true.
* Prevents loading skeletons from flashing too briefly to perceive.
*/
export function useMinDisplayDelay(active: boolean, minMs = 450): boolean {
  const [visible, setVisible] = useState(active);
  const activeSinceRef = useRef<number | null>(active ? Date.now() : null);

  useEffect(() => {
    if (active) {
      if (activeSinceRef.current === null) {
        activeSinceRef.current = Date.now();
      }
      setVisible(true);
      return;
    }

    if (activeSinceRef.current === null) {
      setVisible(false);
      return;
    }

    const elapsed = Date.now() - activeSinceRef.current;
    const remaining = Math.max(0, minMs - elapsed);
    const timer = window.setTimeout(() => {
      activeSinceRef.current = null;
      setVisible(false);
    }, remaining);

    return () => window.clearTimeout(timer);
  }, [active, minMs]);

  return visible;
}
