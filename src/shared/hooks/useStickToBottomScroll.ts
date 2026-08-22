import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from "react";

import {
  isNearBottom,
  scrollElementToBottom,
  STICK_TO_BOTTOM_THRESHOLD_PX,
} from "@/features/pipeline/lib/stickToBottomScroll";

export interface StickToBottomContent {
  historyLength: number;
  hasLiveRow: boolean;
}

interface Options {
  threshold?: number;
  /** Master switch — when false, listeners and stick are idle. */
  enabled?: boolean;
  /**
  * When true, content growth re-sticks while pinned (subject to `applyScroll`).
  * Set false after translate stops so history browsing is not yanked to the tail.
  */
  followContent?: boolean;
  /**
  * When true (default), the hook writes `scrollTop = scrollHeight` on stick.
  * Virtualized Live columns must set false — inflated `getTotalSize()` estimates
  * make DOM scrollHeight overshoot; those columns stick via `scrollToIndex`.
  */
  applyScroll?: boolean;
  /**
  * When true, MutationObserver also watches characterData. Prefer false —
  * live text updates are driven by React props + ResizeObserver;
  * characterData storms burn CPU and race user scroll.
  */
  watchCharacterData?: boolean;
}

/**
* Stick-to-bottom for a scroll container.
*
* Unfocused WebViews (e.g. always-on-top overlay) throttle/pause rAF, which
* used to leave `autoScrollingRef` stuck and miss layout-delayed height
* growth. We clear the guard with timers and re-stick via ResizeObserver +
* window focus.
*
* Citation/anchor jumps call `releasePin()` which also arms an **unpin hold**:
* while held, scroll events must not re-pin just because the viewport is still
* near the bottom at the start of `scrollToIndex`. The hold clears once the
* scroll leaves the bottom, or on `jumpToBottom()`.
*
* Programmatic stick arms `autoScrollingRef` so echo scroll events near the
* bottom are ignored — but a user scroll that leaves the threshold still
* unpins (wheel-up / drag), even during that window.
*/
export function useStickToBottomScroll(
  scrollRef: RefObject<HTMLElement | null>,
  content: StickToBottomContent,
  options: Options = {},
) {
  const threshold = options.threshold ?? STICK_TO_BOTTOM_THRESHOLD_PX;
  const enabled = options.enabled ?? true;
  const followContent = options.followContent ?? true;
  const applyScroll = options.applyScroll ?? true;
  const watchCharacterData = options.watchCharacterData ?? false;

  const pinnedRef = useRef(true);
  const autoScrollingRef = useRef(false);
  /** Blocks near-bottom re-pin after an intentional unpin (citation jump / wheel-up). */
  const unpinHoldRef = useRef(false);
  const lastHistoryLengthRef = useRef(content.historyLength);
  const followContentRef = useRef(followContent);
  followContentRef.current = followContent;
  const applyScrollRef = useRef(applyScroll);
  applyScrollRef.current = applyScroll;

  const [isPinned, setIsPinned] = useState(true);
  const [newSinceUnpinned, setNewSinceUnpinned] = useState(0);

  // Observers (RO / MO / focus) are stable closures — they must read the latest
  // followContent / applyScroll through refs. Layout effects receive fresh closure
  // values on re-render and can use the de-structured variables directly.
  const shouldStick = useCallback(
    () => pinnedRef.current && followContentRef.current,
    [],
  );

  /** Arm the echo-scroll guard for external programmatic scrolls (e.g. scrollToIndex). */
  const beginProgrammaticScroll = useCallback(() => {
    autoScrollingRef.current = true;
    queueMicrotask(() => {
      autoScrollingRef.current = false;
    });
  }, []);

  const scrollToBottom = useCallback(() => {
    const element = scrollRef.current;
    if (!element) return;

    beginProgrammaticScroll();
    scrollElementToBottom(element);
  }, [beginProgrammaticScroll, scrollRef]);

  const pinToBottom = useCallback(() => {
    unpinHoldRef.current = false;
    pinnedRef.current = true;
    setIsPinned(true);
    setNewSinceUnpinned(0);
    lastHistoryLengthRef.current = content.historyLength;
  }, [content.historyLength]);

  const jumpToBottom = useCallback(() => {
    pinToBottom();
    if (applyScrollRef.current) {
      scrollToBottom();
    }
  }, [pinToBottom, scrollToBottom]);

  /**
  * Leave stick-to-bottom for a citation/anchor jump or explicit user intent.
  * Arms unpin-hold so still-near-bottom frames cannot immediately re-pin.
  */
  const releasePin = useCallback(() => {
    unpinHoldRef.current = true;
    pinnedRef.current = false;
    setIsPinned(false);
  }, []);

  const applyScrollPinState = useCallback(
    (element: HTMLElement) => {
      const nearBottom = isNearBottom(element, threshold);

      if (unpinHoldRef.current) {
        // Stay unpinned while the jump is still inside the bottom threshold;
        // once away from the live tail, resume normal pin detection.
        if (!nearBottom) {
          unpinHoldRef.current = false;
        }
        return;
      }

      if (nearBottom === pinnedRef.current) return;

      pinnedRef.current = nearBottom;
      setIsPinned(nearBottom);
      if (nearBottom) {
        setNewSinceUnpinned(0);
        lastHistoryLengthRef.current = content.historyLength;
      }
    },
    [content.historyLength, threshold],
  );

  useEffect(() => {
    if (!enabled) return;

    const element = scrollRef.current;
    if (!element) return;

    const onScroll = () => {
      // Programmatic stick echo: ignore only while still near the bottom.
      // If the user moved away during the auto-scroll window, unpin.
      if (autoScrollingRef.current) {
        if (isNearBottom(element, threshold)) return;
        autoScrollingRef.current = false;
      }

      applyScrollPinState(element);
    };

    /** Wheel-up = intent to read history — unpin immediately (P2). */
    const onWheel = (event: WheelEvent) => {
      if (event.deltaY >= 0) return;
      if (!pinnedRef.current && !autoScrollingRef.current) return;
      releasePin();
    };

    element.addEventListener("scroll", onScroll, { passive: true });
    element.addEventListener("wheel", onWheel, { passive: true });
    return () => {
      element.removeEventListener("scroll", onScroll);
      element.removeEventListener("wheel", onWheel);
    };
  }, [applyScrollPinState, enabled, releasePin, scrollRef, threshold]);

  useLayoutEffect(() => {
    if (!enabled) return;

    const historyLength = content.historyLength;
    const previousHistoryLength = lastHistoryLengthRef.current;

    if (pinnedRef.current && followContent) {
      if (applyScroll) scrollToBottom();
      lastHistoryLengthRef.current = historyLength;
      return;
    }

    if (historyLength > previousHistoryLength) {
      setNewSinceUnpinned(
        (count) => count + (historyLength - previousHistoryLength),
      );
    }
    lastHistoryLengthRef.current = historyLength;
  }, [
    applyScroll,
    content.hasLiveRow,
    content.historyLength,
    enabled,
    followContent,
    scrollToBottom,
  ]);

  // Content height can grow after commit (live text / deferred unfocused
  // layout). ResizeObserver runs after layout — unlike rAF while backgrounded.
  useEffect(() => {
    if (!enabled || !applyScroll) return;
    const element = scrollRef.current;
    if (!element || typeof ResizeObserver === "undefined") return;

    const ro = new ResizeObserver(() => {
      if (shouldStick()) scrollToBottom();
    });

    for (const child of element.children) {
      ro.observe(child);
    }
    // Also observe the scroll container itself for height changes.
    ro.observe(element);

    const mo = new MutationObserver((mutations) => {
      // Only attach new childList nodes to RO — avoid re-observing everything
      // on every characterData tick.
      for (const mutation of mutations) {
        if (mutation.type !== "childList") continue;
        for (const node of mutation.addedNodes) {
          if (node instanceof Element) ro.observe(node);
        }
      }
      if (shouldStick()) scrollToBottom();
    });
    mo.observe(element, {
      childList: true,
      subtree: true,
      characterData: watchCharacterData,
    });

    return () => {
      ro.disconnect();
      mo.disconnect();
    };
  }, [applyScroll, enabled, scrollRef, scrollToBottom, shouldStick, watchCharacterData]);

  // Catch up when the overlay/window is focused again after deferred layout.
  useEffect(() => {
    if (!enabled || !followContent || !applyScroll) return;

    const onFocus = () => {
      if (shouldStick()) scrollToBottom();
    };

    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [applyScroll, enabled, followContent, scrollToBottom, shouldStick]);

  useEffect(() => {
    if (!enabled || content.historyLength > 0 || content.hasLiveRow) return;

    unpinHoldRef.current = false;
    pinnedRef.current = true;
    setIsPinned(true);
    setNewSinceUnpinned(0);
    lastHistoryLengthRef.current = 0;
  }, [content.hasLiveRow, content.historyLength, enabled]);

  return {
    isPinned,
    newSinceUnpinned,
    jumpToBottom,
    releasePin,
    beginProgrammaticScroll,
  };
}
