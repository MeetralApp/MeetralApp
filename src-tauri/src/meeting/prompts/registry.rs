//! Template registry — ship product ids; test-only ids resolve but are not listed.

use super::context::PromptContext;
use super::templates::action_items_only;
use super::templates::decisions_log;
use super::templates::executive_summary;
use super::templates::meeting_brief;
use super::templates::standup;

pub const DEFAULT_SUMMARY_TEMPLATE_ID: &str = "meeting_brief";

pub struct TemplateSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// When false, hidden from `list_summary_templates` (test-only seam ids).
    pub shipped: bool,
    pub build_summary: fn(&PromptContext<'_>) -> String,
    pub build_merge: fn(&PromptContext<'_>, &str) -> String,
}

impl TemplateSpec {
    /// Stable prompt id: `<template>@v2` for telemetry/log-correlation.
    /// Bump the version suffix when a template's contract changes.
    /// (`@v2` = meeting title + context block + terminology rules.)
    pub fn prompt_id(&self) -> String {
        format!("{}@v2", self.id)
    }
}

fn shipped_templates() -> &'static [TemplateSpec] {
    &[
        TemplateSpec {
            id: DEFAULT_SUMMARY_TEMPLATE_ID,
            name: "Meeting brief",
            description:
                "Overview, key points, decisions, and follow-up actions — each point links to transcript sources.",
            shipped: true,
            build_summary: meeting_brief::build_summary,
            build_merge: meeting_brief::build_merge,
        },
        TemplateSpec {
            id: "executive_summary",
            name: "Executive summary",
            description:
                "Concise 3–5 sentence narrative for leadership — strategic framing, no bullet lists.",
            shipped: true,
            build_summary: executive_summary::build_summary,
            build_merge: executive_summary::build_merge,
        },
        TemplateSpec {
            id: "action_items_only",
            name: "Action items",
            description:
                "Task list with owners and deadlines — fastest path to follow-ups.",
            shipped: true,
            build_summary: action_items_only::build_summary,
            build_merge: action_items_only::build_merge,
        },
        TemplateSpec {
            id: "decisions_log",
            name: "Decisions log",
            description:
                "Chronological decision record with revision tracking for audit and compliance.",
            shipped: true,
            build_summary: decisions_log::build_summary,
            build_merge: decisions_log::build_merge,
        },
        TemplateSpec {
            id: "standup",
            name: "Standup",
            description:
                "Yesterday's progress, today's plan, and blockers — for daily syncs.",
            shipped: true,
            build_summary: standup::build_summary,
            build_merge: standup::build_merge,
        },
    ]
}

#[cfg(test)]
fn test_only_templates() -> &'static [TemplateSpec] {
    &[TemplateSpec {
        id: "test_only_brief",
        name: "Test-only brief",
        description: "Registry seam test — not shipped.",
        shipped: false,
        build_summary: meeting_brief::build_summary,
        build_merge: meeting_brief::build_merge,
    }]
}

pub fn list_summary_templates() -> Vec<&'static TemplateSpec> {
    shipped_templates().iter().filter(|t| t.shipped).collect()
}

/// Product + test-only (under `cfg(test)`) template resolution for builders.
pub fn resolve_template(id: &str) -> Option<&'static TemplateSpec> {
    if let Some(spec) = shipped_templates().iter().find(|t| t.id == id) {
        return Some(spec);
    }
    #[cfg(test)]
    {
        test_only_templates().iter().find(|t| t.id == id)
    }
    #[cfg(not(test))]
    {
        None
    }
}

/// Shipped product templates only (generate path).
pub fn is_known_summary_template(id: &str) -> bool {
    shipped_templates().iter().any(|t| t.id == id)
}
