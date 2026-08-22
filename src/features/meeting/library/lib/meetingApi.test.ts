import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import * as meetingApi from "./meetingApi";

describe("meetingApi", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("listMeetingFolders invokes list_meeting_folders", async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    await meetingApi.listMeetingFolders();
    expect(invoke).toHaveBeenCalledWith("list_meeting_folders");
  });

  it("createMeetingFolder passes request payload", async () => {
    await meetingApi.createMeetingFolder("Work", "parent-1");
    expect(invoke).toHaveBeenCalledWith("create_meeting_folder", {
      request: { name: "Work", parentId: "parent-1" },
    });
  });

  it("createMeeting omits empty title/folder as null", async () => {
    await meetingApi.createMeeting();
    expect(invoke).toHaveBeenCalledWith("create_meeting", {
      request: { title: null, folderId: null },
    });
  });

  it("listMeetings and getMeeting use expected commands", async () => {
    await meetingApi.listMeetings("f1", 10, 5);
    expect(invoke).toHaveBeenCalledWith("list_meetings", {
      request: { folderId: "f1", limit: 10, offset: 5 },
    });
    await meetingApi.getMeeting("m1");
    expect(invoke).toHaveBeenCalledWith("get_meeting", { request: { id: "m1" } });
  });

  it("segment list helpers set tail/before flags", async () => {
    await meetingApi.listMeetingSegmentsTail("m1", "outbound", 20);
    expect(invoke).toHaveBeenCalledWith("list_meeting_segments", {
      request: {
        meetingId: "m1",
        direction: "outbound",
        fromSequence: null,
        beforeSequence: null,
        tail: true,
        limit: 20,
      },
    });
    await meetingApi.listMeetingSegmentsBefore("m1", "inbound", 40, 15);
    expect(invoke).toHaveBeenCalledWith("list_meeting_segments", {
      request: {
        meetingId: "m1",
        direction: "inbound",
        fromSequence: null,
        beforeSequence: 40,
        tail: null,
        limit: 15,
      },
    });
  });

  it("search helpers invoke search_segments and search_meetings", async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    await meetingApi.searchSegments({
      query: "budget",
      meetingId: "m1",
      limit: 10,
    });
    expect(invoke).toHaveBeenCalledWith("search_segments", {
      request: {
        query: "budget",
        meetingId: "m1",
        folderId: null,
        limit: 10,
      },
    });
    await meetingApi.searchMeetings({ query: "budget", folderId: "f1" });
    expect(invoke).toHaveBeenCalledWith("search_meetings", {
      request: {
        query: "budget",
        folderId: "f1",
        limit: null,
      },
    });
  });

  it("summary commands map correctly", async () => {
    await meetingApi.getMeetingSummary("m1");
    expect(invoke).toHaveBeenCalledWith("get_meeting_summary", {
      request: { id: "m1" },
    });
    await meetingApi.generateMeetingSummary("m1", "brief", "en");
    expect(invoke).toHaveBeenCalledWith("generate_meeting_summary", {
      request: { id: "m1", templateId: "brief", language: "en" },
    });
    await meetingApi.updateMeetingSummary("m1", '{"type":"doc","content":[]}');
    expect(invoke).toHaveBeenCalledWith("update_meeting_summary", {
      request: {
        meetingId: "m1",
        docJson: '{"type":"doc","content":[]}',
      },
    });
  });

  it("meeting lifecycle and folder mutations", async () => {
    await meetingApi.getActiveMeeting();
    expect(invoke).toHaveBeenCalledWith("get_active_meeting");
    await meetingApi.endMeeting("m1");
    expect(invoke).toHaveBeenCalledWith("end_meeting", { request: { id: "m1" } });
    await meetingApi.renameMeeting("m1", "New");
    expect(invoke).toHaveBeenCalledWith("rename_meeting", {
      request: { id: "m1", title: "New" },
    });
    await meetingApi.moveMeeting("m1", "f2");
    expect(invoke).toHaveBeenCalledWith("move_meeting", {
      request: { id: "m1", folderId: "f2" },
    });
    await meetingApi.deleteMeeting("m1");
    expect(invoke).toHaveBeenCalledWith("delete_meeting", { request: { id: "m1" } });
    await meetingApi.renameMeetingFolder("f1", "Renamed");
    expect(invoke).toHaveBeenCalledWith("rename_meeting_folder", {
      request: { id: "f1", name: "Renamed" },
    });
    await meetingApi.deleteMeetingFolder("f1");
    expect(invoke).toHaveBeenCalledWith("delete_meeting_folder", {
      request: { id: "f1" },
    });
    await meetingApi.reorderMeetingFolders(["a", "b"]);
    expect(invoke).toHaveBeenCalledWith("reorder_meeting_folders", {
      request: { folderIds: ["a", "b"] },
    });
  });

  it("neighbors templates languages and list segments", async () => {
    await meetingApi.getSegmentNeighbors("m1", "s1");
    expect(invoke).toHaveBeenCalledWith("get_segment_neighbors", {
      request: { meetingId: "m1", segmentId: "s1" },
    });
    await meetingApi.listSummaryTemplates();
    expect(invoke).toHaveBeenCalledWith("list_summary_templates_cmd");
    await meetingApi.listSummaryLanguages();
    expect(invoke).toHaveBeenCalledWith("list_summary_languages_cmd");
    await meetingApi.listMeetingSegments("m1", "outbound", 5, 100);
    expect(invoke).toHaveBeenCalledWith("list_meeting_segments", {
      request: {
        meetingId: "m1",
        direction: "outbound",
        fromSequence: 5,
        beforeSequence: null,
        tail: null,
        limit: 100,
      },
    });
  });
});
