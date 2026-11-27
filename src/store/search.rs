use anyhow::Result;
use rusqlite::params;
use unicode_normalization::UnicodeNormalization;

use super::{PlanSummary, SearchResults, Store, TaskRow};

/// Lowercase, strip combining marks and map đ/Đ to d, so that text and
/// queries compare without regard to Vietnamese diacritics.
pub fn fold(s: &str) -> String {
    s.nfd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'đ' { 'd' } else { c })
        .collect()
}

/// `"tok"*` per whitespace token, AND-ed. None if nothing searchable.
fn fts_query(q: &str) -> Option<String> {
    let terms: Vec<String> = fold(q)
        .split_whitespace()
        .filter(|t| t.chars().any(char::is_alphanumeric))
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

pub(super) const REF_SEP: char = '\u{1f}';

impl Store {
    pub fn search(&self, q: &str, limit: usize) -> Result<SearchResults> {
        let empty = SearchResults {
            plans: vec![],
            tasks: vec![],
            topics: vec![],
        };
        let Some(fts) = fts_query(q) else {
            return Ok(empty);
        };
        let limit_i = i64::try_from(limit).unwrap_or(i64::MAX);

        let (plan_ids, task_refs) = {
            let conn = self.conn();
            let mut stmt = conn.prepare(
                "SELECT ref FROM search_fts WHERE search_fts MATCH ?1 AND kind = ?2
                 ORDER BY rank LIMIT ?3",
            )?;
            let mut refs = |kind: &str| -> Result<Vec<String>> {
                Ok(stmt
                    .query_map(params![fts, kind, limit_i], |r| r.get(0))?
                    .collect::<rusqlite::Result<_>>()?)
            };
            (refs("plan")?, refs("task")?)
        };

        let mut plans: Vec<PlanSummary> = Vec::new();
        for id in &plan_ids {
            if let Some(p) = self.plan_summary(id)? {
                plans.push(p);
            }
        }
        let mut tasks: Vec<TaskRow> = Vec::new();
        for r in &task_refs {
            if let Some((plan_id, task_id)) = r.split_once(REF_SEP)
                && let Some(t) = self.task_row(plan_id, task_id)?
            {
                tasks.push(t);
            }
        }

        let needle = fold(q.trim());
        let mut topics = Vec::new();
        if !needle.is_empty() {
            for t in self.topics()? {
                let matches = t.tag.contains(&needle) || {
                    self.tax.tags.get(&t.tag).is_some_and(|i| {
                        fold(&i.label_en).contains(&needle) || fold(&i.label_vi).contains(&needle)
                    })
                };
                if matches {
                    topics.push(t);
                    if topics.len() >= limit {
                        break;
                    }
                }
            }
        }
        Ok(SearchResults {
            plans,
            tasks,
            topics,
        })
    }

    /// Plan or task tag keys -> "label_en label_vi ..." for indexing.
    pub(super) fn tag_labels<'a>(&self, tags: impl Iterator<Item = &'a String>) -> String {
        tags.filter_map(|t| self.tax.tags.get(t))
            .map(|i| format!("{} {}", i.label_en, i.label_vi))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_strips_diacritics_and_d_bar() {
        assert_eq!(fold("Đăng Nhập"), "dang nhap");
        assert_eq!(fold("Bảo mật"), "bao mat");
        // decomposed input folds the same way
        assert_eq!(fold("Ba\u{0309}o ma\u{0323}t"), "bao mat");
    }

    #[test]
    fn query_is_prefix_and_escaped() {
        assert_eq!(fts_query("dang nh").unwrap(), "\"dang\"* \"nh\"*");
        assert_eq!(fts_query("a\"b").unwrap(), "\"a\"\"b\"*");
        assert_eq!(fts_query("  - * "), None);
        assert_eq!(fts_query(""), None);
    }
}
