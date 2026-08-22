export type OverlayTailRow = {
  id: string;
  direction: "outbound" | "inbound";
  sourceText: string;
  translatedText: string;
  live?: boolean;
};

export const MOCK_OVERLAY_ROWS: OverlayTailRow[] = [
  {
    id: "mock-1",
    direction: "outbound",
    sourceText: "Can we start the meeting now?",
    translatedText: "Chúng ta có thể bắt đầu cuộc họp chưa?",
  },
  {
    id: "mock-2",
    direction: "inbound",
    sourceText: "Yes, the room is ready.",
    translatedText: "Vâng, phòng đã sẵn sàng.",
  },
  {
    id: "mock-3",
    direction: "outbound",
    sourceText: "I'll share the agenda first.",
    translatedText: "Tôi sẽ chia sẻ chương trình trước.",
    live: true,
  },
];
