# Working knowledge

This tree is **not** product or architecture source of truth.

Living SSOT: `docs/architecture/`, `docs/features/`, `docs/integrations/`, `docs/development/`.

| Folder | Use for | After |
|--------|---------|--------|
| `plans/` | In-flight design for a change you are about to make | Fold into living docs and **delete** the plan |
| `research/` | Experiments, vendor comparisons, alternatives, open questions | Extract any shipped fact into living docs; leave or delete the rest |
| `bugs/` | Investigations, repro notes, hypotheses | Close out when the fix ships; do not leave a second architecture narrative here |

Agents **must not** implement from these folders. A plan may be wrong, stale, or describe an option that was rejected.

Keep notes short. Do not grow a versioned plan archive; fold shipped work into living docs and delete the plan.
