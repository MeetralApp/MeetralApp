import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import { APP_EVENTS } from "@/shared/lib/events";

const getMeeting = vi.fn();
const listMeetingSegments = vi.fn();
const getMeetingSummary = vi.fn();
const generateMeetingSummary = vi.fn();
const getSummaryGenerationStatus = vi.fn();

const eventHandlers: Record<string, Array<(event: { payload: unknown }) => void>> =
  {};

function emitEvent<T>(eventName: string, payload: T) {
  for (const handler of eventHandlers[eventName] ?? []) {
    handler({ payload });
  }
}

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, handler: (event: { payload: unknown }) => void) => {
    eventHandlers[eventName] ??= [];
    eventHandlers[eventName].push(handler);
    return Promise.resolve(() => {});
  }),
}));

vi.mock("@/features/meeting/library/lib/meetingApi", () => ({
  getMeeting: (...args: unknown[]) => getMeeting(...args),
  listMeetingSegments: (...args: unknown[]) => listMeetingSegments(...args),
  getMeetingSummary: (...args: unknown[]) => getMeetingSummary(...args),
  generateMeetingSummary: (...args: unknown[]) => generateMeetingSummary(...args),
  updateMeetingSummary: vi.fn(),
  getSummaryGenerationStatus: (...args: unknown[]) =>
    getSummaryGenerationStatus(...args),
}));

import { useMeetingDetail } from "./useMeetingDetail";

describe("useMeetingDetail", () => {
  beforeEach(() => {
    getMeeting.mockReset();
    listMeetingSegments.mockReset();
    getMeetingSummary.mockReset();
    generateMeetingSummary.mockReset();
    getSummaryGenerationStatus.mockReset().mockResolvedValue(null);
    for (const key of Object.keys(eventHandlers)) {
      delete eventHandlers[key];
    }
  });

  it("clears state when meetingId is null", async () => {
    const { result } = renderHook(() => useMeetingDetail(null));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.meeting).toBeNull();
    expect(result.current.segments).toEqual([]);
    expect(getMeeting).not.toHaveBeenCalled();
  });

  it("loads meeting, segments, and summary", async () => {
    getMeeting.mockResolvedValue({ id: "m1", title: "Standup" });
    listMeetingSegments.mockResolvedValue({
      segments: [{ id: "s1", meetingId: "m1", direction: "outbound", sequence: 1 }],
      hasMore: false,
    });
    getMeetingSummary.mockResolvedValue({
      meetingId: "m1",
      templateId: "default",
      summaryLanguage: "en",
      generatedJson: '{"type":"doc","content":[{"type":"paragraph"}]}',
      generatedText: "summary",
      generatedAtMs: 1,
    });

    const { result } = renderHook(() => useMeetingDetail("m1"));
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(getMeeting).toHaveBeenCalledWith("m1");
    expect(listMeetingSegments).toHaveBeenCalledWith(
      "m1",
      undefined,
      undefined,
      2000,
    );
    expect(getMeetingSummary).toHaveBeenCalledWith("m1");
    expect(result.current.meeting?.title).toBe("Standup");
    expect(result.current.segments).toHaveLength(1);
    expect(result.current.summary?.meetingId).toBe("m1");
    expect(result.current.summary?.generatedText).toBe("summary");
  });

  it("sets error when load fails", async () => {
    getMeeting.mockRejectedValue(new Error("boom"));
    listMeetingSegments.mockResolvedValue({ segments: [], hasMore: false });

    const { result } = renderHook(() => useMeetingDetail("m1"));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toContain("boom");
  });

  it("restores in-flight summary state on mount", async () => {
    getMeeting.mockResolvedValue({ id: "m1", title: "Standup" });
    listMeetingSegments.mockResolvedValue({ segments: [], hasMore: false });
    getMeetingSummary.mockResolvedValue(null);
    getSummaryGenerationStatus.mockResolvedValue({
      meetingId: "m1",
      phase: "chunk",
      current: 2,
      total: 5,
      startedAtMs: 1000,
    });

    const { result } = renderHook(() => useMeetingDetail("m1"));
    await waitFor(() => expect(result.current.summaryLoading).toBe(true));
    expect(result.current.summaryProgress).toEqual({
      label: "Reading…",
      detail: "Reading transcript (part 2 of 5)…",
    });
  });

  it("refetches summary on summary-done event", async () => {
    getMeeting.mockResolvedValue({ id: "m1", title: "Standup" });
    listMeetingSegments.mockResolvedValue({ segments: [], hasMore: false });
    getMeetingSummary
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({
        meetingId: "m1",
        templateId: "default",
        summaryLanguage: "en",
        generatedJson: "{}",
        generatedText: "final",
        generatedAtMs: 2,
      });
    getSummaryGenerationStatus.mockResolvedValue({
      meetingId: "m1",
      phase: "chunk",
      current: 1,
      total: 5,
      startedAtMs: 1,
    });

    const { result } = renderHook(() => useMeetingDetail("m1"));
    await waitFor(() => expect(result.current.summaryLoading).toBe(true));

    act(() => {
      emitEvent(APP_EVENTS.summaryDone, { meetingId: "m1" });
    });

    await waitFor(() => expect(result.current.summaryLoading).toBe(false));
    expect(result.current.summary?.generatedText).toBe("final");
    expect(result.current.summaryProgress).toBeNull();
  });

  it("surfaces summary-error event as inline error", async () => {
    getMeeting.mockResolvedValue({ id: "m1", title: "Standup" });
    listMeetingSegments.mockResolvedValue({ segments: [], hasMore: false });
    getMeetingSummary.mockResolvedValue(null);
    getSummaryGenerationStatus.mockResolvedValue({
      meetingId: "m1",
      phase: "starting",
      current: 0,
      total: 0,
      startedAtMs: 1,
    });

    const { result } = renderHook(() => useMeetingDetail("m1"));
    await waitFor(() => expect(result.current.summaryLoading).toBe(true));

    act(() => {
      emitEvent(APP_EVENTS.summaryError, {
        meetingId: "m1",
        message: "LLM boom",
      });
    });

    await waitFor(() => expect(result.current.summaryLoading).toBe(false));
    expect(result.current.summaryError).toContain("LLM boom");
    expect(result.current.summaryProgress).toBeNull();
  });

  it("maps summary_already_running to friendly inline error", async () => {
    getMeeting.mockResolvedValue({ id: "m1", title: "Standup" });
    listMeetingSegments.mockResolvedValue({ segments: [], hasMore: false });
    getMeetingSummary.mockResolvedValue(null);
    getSummaryGenerationStatus.mockResolvedValue(null);
    generateMeetingSummary.mockRejectedValue("summary_already_running");

    const { result } = renderHook(() => useMeetingDetail("m1"));
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.generateSummary("default", "en");
    });

    expect(result.current.summaryError).toBe(
      "Summary generation is already running for this meeting.",
    );
  });

  it("updates progress from summary-progress event while generating", async () => {
    getMeeting.mockResolvedValue({ id: "m1", title: "Standup" });
    listMeetingSegments.mockResolvedValue({ segments: [], hasMore: false });
    getMeetingSummary.mockResolvedValue(null);
    getSummaryGenerationStatus.mockResolvedValue(null);

    let resolveGenerate: (value: unknown) => void = () => {};
    generateMeetingSummary.mockImplementation(
      () => new Promise((resolve) => {
        resolveGenerate = resolve;
      }),
    );

    const { result } = renderHook(() => useMeetingDetail("m1"));
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      void result.current.generateSummary("default", "en");
    });
    await waitFor(() => expect(result.current.summaryLoading).toBe(true));

    act(() => {
      emitEvent(APP_EVENTS.summaryProgress, {
        meetingId: "m1",
        phase: "chunk",
        current: 1,
        total: 3,
      });
    });

    await waitFor(() =>
      expect(result.current.summaryProgress?.detail).toContain("part 1 of 3"),
    );

    act(() => {
      resolveGenerate({
        meetingId: "m1",
        templateId: "default",
        summaryLanguage: "en",
        generatedJson: "{}",
        generatedText: "ok",
        generatedAtMs: 1,
      });
    });

    await waitFor(() => expect(result.current.summaryLoading).toBe(false));
  });
});
