import { describe, expect, it, vi, beforeEach } from "vitest";

type Handler = (event: { payload: unknown }) => void;

interface Registration {
  event: string;
  handler: Handler;
  resolve: (fn: () => void) => void;
}

let registrations: Registration[] = [];

vi.mock("@tauri-apps/api/event", () => ({
  listen: (event: string, handler: Handler) => {
    let resolve!: (fn: () => void) => void;
    const promise = new Promise<() => void>((res) => {
      resolve = res;
    });
    registrations.push({ event, handler, resolve });
    return promise;
  },
}));

import { guardUnlisten, listenSafe } from "./listenSafe";

const flush = () => new Promise<void>((r) => setTimeout(r, 0));

describe("listenSafe", () => {
  beforeEach(() => {
    registrations = [];
  });

  it("dispose after resolve calls unlisten once", async () => {
    const unlisten = vi.fn();
    const dispose = listenSafe<string>("transcript", vi.fn());
    registrations[0].resolve(unlisten);
    await flush();

    dispose();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("dispose before resolve self-unlistens on resolve (cleanup race)", async () => {
    const unlisten = vi.fn();
    const dispose = listenSafe<string>("transcript", vi.fn());
    dispose();

    registrations[0].resolve(unlisten);
    await flush();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("double dispose is a no-op", async () => {
    const unlisten = vi.fn();
    const dispose = listenSafe<string>("transcript", vi.fn());
    registrations[0].resolve(unlisten);
    await flush();

    dispose();
    dispose();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("rejected registration does not throw and dispose stays safe", async () => {
    const dispose = guardUnlisten(Promise.reject(new Error("ipc down")));
    dispose();
    await flush();
  });
});
