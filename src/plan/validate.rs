use std::collections::HashSet;

use super::model::{Plan, Warning};
use crate::taxonomy::Taxonomy;

const REQUIRED_SECTIONS: [&str; 5] = ["01.", "02.", "03.", "04.", "05."];

/// Semantic checks that need the taxonomy or the whole plan. Parse-time
/// warnings stay in `plan.warnings`; callers merge both lists.
pub fn validate(plan: &Plan, tax: &Taxonomy) -> Vec<Warning> {
    let mut out = Vec::new();

    let mut seen = HashSet::new();
    let mut reported = HashSet::new();
    for t in &plan.tasks {
        if !seen.insert(t.id.as_str()) && reported.insert(t.id.as_str()) {
            out.push(Warning::new(
                "duplicate_task_id",
                format!("task id {} appears more than once", t.id),
            ));
        }
    }

    let mut tags: Vec<&String> = plan.tags.iter().collect();
    for t in &plan.tasks {
        tags.extend(t.tags.iter());
    }
    let mut reported = HashSet::new();
    for tag in tags {
        if !tax.contains(tag) && reported.insert(tag.as_str()) {
            out.push(Warning::new(
                "unknown_tag",
                format!("tag {tag} is not in the taxonomy"),
            ));
        }
    }

    if plan.structured {
        for prefix in REQUIRED_SECTIONS {
            if !plan
                .sections
                .iter()
                .any(|s| s.title.trim_start().starts_with(prefix))
            {
                out.push(Warning::new(
                    "missing_section",
                    format!("structured plan has no section {prefix}"),
                ));
            }
        }
    }
    out
}
