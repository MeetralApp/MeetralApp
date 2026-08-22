import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";

/**
* Wraps any promise of an UnlistenFn (event listen, window onMoved, ...) and
* returns a dispose function that is safe to call before the promise resolves.
* Without this, a React effect cleanup that runs first leaves the listener
* registered forever — the classic StrictMode/remount listener leak.
*/
export function guardUnlisten(registration: Promise<UnlistenFn>): () => void {
  let disposed = false;
  let unlisten: UnlistenFn | undefined;

  void registration
    .then((fn) => {
      if (disposed) {
        fn();
        return;
      }
      unlisten = fn;
    })
    .catch(() => {
    // Registration failed (IPC error): nothing to unlisten.
    });

  return () => {
    if (disposed) return;
    disposed = true;
    if (unlisten) {
      unlisten();
      unlisten = undefined;
    }
  };
}

/** `listen()` with race-safe disposal; the only way this app registers events. */
export function listenSafe<T>(
  eventName: string,
  handler: EventCallback<T>,
): () => void {
  return guardUnlisten(listen<T>(eventName, handler));
}
