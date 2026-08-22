import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { useStickToBottomScroll } from "./useStickToBottomScroll";

function createScrollContainer() {
  const element = document.createElement("div");
  Object.defineProperty(element, "scrollHeight", {
    configurable: true,
    get: () => 1000,
  });
  Object.defineProperty(element, "clientHeight", {
    configurable: true,
    get: () => 400,
  });
  let scrollTop = 600;
  Object.defineProperty(element, "scrollTop", {
    configurable: true,
    get: () => scrollTop,
    set: (value: number) => {
      scrollTop = value;
    },
  });
  return element;
}

describe("useStickToBottomScroll", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("auto-scrolls when pinned and content grows", () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { rerender } = renderHook(
      ({ historyLength }) =>
        useStickToBottomScroll(scrollRef, {
          historyLength,
          hasLiveRow: false,
        }),
      { initialProps: { historyLength: 1 } },
    );

    expect(scrollRef.current.scrollTop).toBe(1000);

    rerender({ historyLength: 2 });
    expect(scrollRef.current.scrollTop).toBe(1000);
  });

  it("stays put when unpinned and content grows", async () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { result, rerender } = renderHook(
      ({ historyLength }) =>
        useStickToBottomScroll(scrollRef, {
          historyLength,
          hasLiveRow: true,
        }),
      { initialProps: { historyLength: 1 } },
    );

    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      scrollRef.current.scrollTop = 100;
      scrollRef.current.dispatchEvent(new Event("scroll"));
    });

    expect(result.current.isPinned).toBe(false);
    expect(scrollRef.current.scrollTop).toBe(100);

    rerender({ historyLength: 2 });
    expect(scrollRef.current.scrollTop).toBe(100);
    expect(result.current.newSinceUnpinned).toBe(1);
  });

  it("jumpToBottom re-pins and scrolls to end", async () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { result } = renderHook(() =>
      useStickToBottomScroll(scrollRef, {
        historyLength: 3,
        hasLiveRow: true,
      }),
    );

    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      scrollRef.current.scrollTop = 100;
      scrollRef.current.dispatchEvent(new Event("scroll"));
    });
    expect(result.current.isPinned).toBe(false);

    act(() => {
      result.current.jumpToBottom();
    });

    expect(result.current.isPinned).toBe(true);
    expect(result.current.newSinceUnpinned).toBe(0);
    expect(scrollRef.current.scrollTop).toBe(1000);
  });

  it("releasePin holds unpin while still near the bottom", async () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { result } = renderHook(() =>
      useStickToBottomScroll(scrollRef, {
        historyLength: 3,
        hasLiveRow: true,
      }),
    );

    await act(async () => {
      await Promise.resolve();
    });

    expect(scrollRef.current.scrollTop).toBe(1000);
    expect(result.current.isPinned).toBe(true);

    act(() => {
      result.current.releasePin();
    });
    expect(result.current.isPinned).toBe(false);

    act(() => {
      scrollRef.current.dispatchEvent(new Event("scroll"));
    });
    expect(result.current.isPinned).toBe(false);

    act(() => {
      scrollRef.current.scrollTop = 100;
      scrollRef.current.dispatchEvent(new Event("scroll"));
    });
    expect(result.current.isPinned).toBe(false);

    act(() => {
      scrollRef.current.scrollTop = 1000;
      scrollRef.current.dispatchEvent(new Event("scroll"));
    });
    expect(result.current.isPinned).toBe(true);
  });

  it("does not ignore user unpin scroll that races stick autoScrollingRef", async () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { result } = renderHook(() =>
      useStickToBottomScroll(scrollRef, {
        historyLength: 5,
        hasLiveRow: true,
      }),
    );

    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      scrollRef.current.scrollTop = 100;
      scrollRef.current.dispatchEvent(new Event("scroll"));
    });
    expect(result.current.isPinned).toBe(false);

    act(() => {
      result.current.jumpToBottom();
      scrollRef.current.scrollTop = 100;
      scrollRef.current.dispatchEvent(new Event("scroll"));
    });

    expect(result.current.isPinned).toBe(false);
    expect(scrollRef.current.scrollTop).toBe(100);
  });

  it("wheel-up releases pin immediately", async () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { result } = renderHook(() =>
      useStickToBottomScroll(scrollRef, {
        historyLength: 5,
        hasLiveRow: true,
      }),
    );

    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.isPinned).toBe(true);

    act(() => {
      scrollRef.current.dispatchEvent(
        new WheelEvent("wheel", { deltaY: -40 }),
      );
    });

    expect(result.current.isPinned).toBe(false);
  });

  it("does not auto-stick when followContent is false", async () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { rerender } = renderHook(
      ({ historyLength, followContent }) =>
        useStickToBottomScroll(
          scrollRef,
          { historyLength, hasLiveRow: false },
          { followContent },
        ),
      { initialProps: { historyLength: 2, followContent: true } },
    );

    await act(async () => {
      await Promise.resolve();
    });
    expect(scrollRef.current.scrollTop).toBe(1000);

    // Stop following while still pinned (translate stopped).
    rerender({ historyLength: 2, followContent: false });
    act(() => {
      scrollRef.current.scrollTop = 100;
    });

    rerender({ historyLength: 3, followContent: false });
    expect(scrollRef.current.scrollTop).toBe(100);
  });

  it("jumpToBottom with applyScroll false only re-pins (no DOM scroll)", async () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { result } = renderHook(() =>
      useStickToBottomScroll(
        scrollRef,
        { historyLength: 5, hasLiveRow: true },
        { applyScroll: false },
      ),
    );

    await act(async () => {
      await Promise.resolve();
    });

    act(() => {
      scrollRef.current.scrollTop = 100;
      scrollRef.current.dispatchEvent(new Event("scroll"));
    });
    expect(result.current.isPinned).toBe(false);

    act(() => {
      result.current.jumpToBottom();
    });

    expect(result.current.isPinned).toBe(true);
    expect(scrollRef.current.scrollTop).toBe(100);
  });

  it("observers do not stick when followContent is false while pinned", async () => {
    const scrollRef = { current: createScrollContainer() };
    document.body.appendChild(scrollRef.current);

    const { rerender } = renderHook(
      ({ followContent }) =>
        useStickToBottomScroll(
          scrollRef,
          { historyLength: 5, hasLiveRow: false },
          { followContent },
        ),
      { initialProps: { followContent: true } },
    );

    await act(async () => {
      await Promise.resolve();
    });
    expect(scrollRef.current.scrollTop).toBe(1000);

    // Disable follow, scroll up, then trigger a resize-observer-equivalent
    // by re-rendering with new content while pinned.
    rerender({ followContent: false });
    act(() => {
      scrollRef.current.scrollTop = 100;
    });
    // Pin is still true (user never dispatched a scroll far enough).
    // Content growth should NOT stick — shouldStick() is false because followContent is off.
    rerender({ followContent: false });
    expect(scrollRef.current.scrollTop).toBe(100);
  });
});
