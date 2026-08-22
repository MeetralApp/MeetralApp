import type { JSONContent } from "@tiptap/core";

import SummaryDocEditor from "./SummaryDocEditor";
import type { CitationAttrs } from "../lib/summaryDoc/citationExtension";
import type { TranscriptSegment } from "@/features/meeting/library/lib/meetingTypes";

interface Props {
  docJson: string;
  segments: TranscriptSegment[];
  onCitationClick?: (attrs: CitationAttrs) => void;
}

export default function SummaryDocView({
  docJson,
  segments,
  onCitationClick,
}: Props) {
  let doc: JSONContent;
  try {
    doc = JSON.parse(docJson) as JSONContent;
  } catch {
    return (
      <p className="m-0 text-sm text-muted-foreground">
        Could not load edited summary document.
      </p>
    );
  }

  return (
    <SummaryDocEditor
      key={docJson.slice(0, 64)}
      initialDoc={doc}
      segments={segments}
      editable={false}
      onCitationClick={onCitationClick}
      className="border-0 bg-transparent"
    />
  );
}
