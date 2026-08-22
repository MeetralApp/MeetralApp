import { useEffect, useState, type ReactNode } from "react";

import { ClockContext } from "./clockContext";

export function ClockProvider({ children }: { children: ReactNode }) {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const tick = () => setNow(Date.now());
    tick();
    const id = window.setInterval(tick, 1000);
    return () => window.clearInterval(id);
  }, []);

  return (
    <ClockContext.Provider value={{ now }}>{children}</ClockContext.Provider>
  );
}
