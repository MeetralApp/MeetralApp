import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { baseConfig } from "@/test/fixtures/config";
import type { MeetingRecord } from "@/features/meeting/library/lib/meetingTypes";
import { TooltipProvider } from "@/shared/ui/tooltip";
import SessionHeaderHub from "./SessionHeaderHub";

const meeting: MeetingRecord = {
  id: "m1",
  folderId: null,
  title: "Standup",
  myLanguage: "en",
  meetingLanguage: "vi",
  startedAtMs: 1,
  endedAtMs: null,
  status: "live",
};

const notesCapableConfig = {
  ...baseConfig,
  aiProvider: "soniox" as const,
};

function renderHub(ui: ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

describe("SessionHeaderHub", () => {
  it("labels the session chip with meeting and language pair", () => {
    renderHub(
      <SessionHeaderHub
        meeting={meeting}
        myLanguage="en"
        meetingLanguage="vi"
        showSonioxContext={false}
        config={baseConfig}
        onRename={vi.fn()}
        onEnd={vi.fn()}
        onOpenLanguages={vi.fn()}
        onSave={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(
      screen.getByRole("button", {
        name: /Meeting Standup\. Languages English and Vietnamese\. Open session menu/i,
      }),
    ).toBeTruthy();
  });

  it("notes locked languages in the chip aria when translating", () => {
    renderHub(
      <SessionHeaderHub
        meeting={null}
        myLanguage="en"
        meetingLanguage="vi"
        languagesLocked
        showSonioxContext={false}
        config={baseConfig}
        onRename={vi.fn()}
        onEnd={vi.fn()}
        onOpenLanguages={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    const trigger = screen.getByRole("button", {
      name: /Languages English and Vietnamese\. Open session menu/i,
    });
    expect(trigger.getAttribute("aria-label")).not.toMatch(/Meeting /);
    // Locked state is reflected on the chip (opacity + menu button title when open).
    expect(trigger.className).toMatch(/opacity-80/);
  });

  it("locks Interpreter/Notes while a meeting is live even when the pipeline is idle", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);

    renderHub(
      <SessionHeaderHub
        meeting={meeting}
        myLanguage="en"
        meetingLanguage="vi"
        languagesLocked={false}
        showSonioxContext={false}
        config={notesCapableConfig}
        onRename={vi.fn()}
        onEnd={vi.fn()}
        onOpenLanguages={vi.fn()}
        onSave={onSave}
      />,
    );

    fireEvent.pointerDown(
      screen.getByRole("button", {
        name: /Meeting Standup\. Languages English and Vietnamese\. Open session menu/i,
      }),
    );

    const interpreter = await screen.findByRole("button", {
      name: "Interpreter",
    });
    const notes = screen.getByRole("button", { name: "Notes" });
    expect((interpreter as HTMLButtonElement).disabled).toBe(true);
    expect((notes as HTMLButtonElement).disabled).toBe(true);
    expect(
      screen.getByText("End meeting to switch Interpreter / Notes."),
    ).toBeTruthy();

    fireEvent.click(notes);
    expect(onSave).not.toHaveBeenCalled();
  });

  it("captures interpreter prefs when switching to Notes without clobbering meeting language", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);

    renderHub(
      <SessionHeaderHub
        meeting={null}
        myLanguage="vi"
        meetingLanguage="en"
        languagesLocked={false}
        showSonioxContext={false}
        config={{
          ...notesCapableConfig,
          sessionMode: "interpreter",
          myLanguage: "vi",
          meetingLanguage: "en",
          interpreterMyLanguage: "vi",
          interpreterMeetingLanguage: "en",
          notesLanguage: "ja",
          outboundMode: "textOnly",
          inboundMode: "translated",
          interpreterOutboundMode: "textOnly",
          interpreterInboundMode: "translated",
        }}
        onRename={vi.fn()}
        onEnd={vi.fn()}
        onOpenLanguages={vi.fn()}
        onSave={onSave}
      />,
    );

    fireEvent.pointerDown(
      screen.getByRole("button", {
        name: /Languages Vietnamese and English\. Open session menu/i,
      }),
    );
    fireEvent.click(await screen.findByRole("button", { name: "Notes" }));

    expect(onSave).toHaveBeenCalled();
    const payload = onSave.mock.calls[0]![0] as {
      sessionMode: string;
      interpreterMyLanguage: string;
      interpreterMeetingLanguage: string;
      interpreterOutboundMode: string;
      notesLanguage: string;
      meetingLanguage: string;
    };
    expect(payload.sessionMode).toBe("notes");
    expect(payload.interpreterMyLanguage).toBe("vi");
    expect(payload.interpreterMeetingLanguage).toBe("en");
    expect(payload.interpreterOutboundMode).toBe("textOnly");
    expect(payload.notesLanguage).toBe("ja");
    expect(payload.meetingLanguage).toBe("en");
  });

  it("restores Interpreter Custom voice mode when leaving Notes", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);

    renderHub(
      <SessionHeaderHub
        meeting={null}
        myLanguage="ja"
        meetingLanguage="ja"
        languagesLocked={false}
        showSonioxContext={false}
        config={{
          ...notesCapableConfig,
          sessionMode: "notes",
          myLanguage: "ja",
          meetingLanguage: "ja",
          notesLanguage: "ja",
          // Active clamped by Notes:
          outboundMode: "originalAudio",
          inboundMode: "originalAudio",
          outboundVoiceOutput: "providerNative",
          inboundVoiceOutput: "providerNative",
          // Stash must survive:
          interpreterMyLanguage: "vi",
          interpreterMeetingLanguage: "en",
          interpreterOutboundMode: "translated",
          interpreterInboundMode: "translated",
          interpreterOutboundVoiceOutput: "custom",
          interpreterInboundVoiceOutput: "providerNative",
        }}
        onRename={vi.fn()}
        onEnd={vi.fn()}
        onOpenLanguages={vi.fn()}
        onSave={onSave}
      />,
    );

    fireEvent.pointerDown(
      screen.getByRole("button", {
        name: /Language Japanese\. Open session menu/i,
      }),
    );
    fireEvent.click(await screen.findByRole("button", { name: "Interpreter" }));

    expect(onSave).toHaveBeenCalled();
    const payload = onSave.mock.calls[0]![0] as {
      sessionMode: string;
      myLanguage: string;
      meetingLanguage: string;
      outboundMode: string;
      outboundVoiceOutput: string;
      interpreterOutboundMode: string;
      interpreterOutboundVoiceOutput: string;
      notesLanguage: string;
    };
    expect(payload.sessionMode).toBe("interpreter");
    expect(payload.myLanguage).toBe("vi");
    expect(payload.meetingLanguage).toBe("en");
    expect(payload.outboundMode).toBe("translated");
    expect(payload.outboundVoiceOutput).toBe("custom");
    expect(payload.interpreterOutboundMode).toBe("translated");
    expect(payload.interpreterOutboundVoiceOutput).toBe("custom");
    expect(payload.notesLanguage).toBe("ja");
  });
});
