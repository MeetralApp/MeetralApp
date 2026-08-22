#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlushReason {
    TurnComplete,
    SegmentFastLane,
    OpenAiIdleSafety,
    MajorRewrite,
    SentenceBoundary,
}

impl FlushReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::TurnComplete => "turn",
            Self::SegmentFastLane => "segment",
            Self::OpenAiIdleSafety => "openai_idle",
            Self::MajorRewrite => "major_rewrite",
            Self::SentenceBoundary => "sentence_boundary",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReconcileAction {
    NoOp,
    AppendSuffix(String),
    MajorRewrite { remainder: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_rewrite_flush_reason_string() {
        assert_eq!(FlushReason::MajorRewrite.as_str(), "major_rewrite");
    }
}
