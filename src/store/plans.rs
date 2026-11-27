use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, ToSql, params};

use super::search::REF_SEP;
use super::{
    Edge, EdgeKind, Graph, PlanFilter, PlanSummary, ProjectSummary, Store, TaskFilter, TaskRow,
    TopicCount,
};
use crate::plan::{Plan, RelationKind, Section, Task};

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn kind_str(k: RelationKind) -> &'static str {
    match k {
        RelationKind::DependsOn => "depends_on",
        RelationKind::Extends => "extends",
        RelationKind::RelatesTo => "relates_to",
    }
}

fn kind_from_str(s: &str) -> RelationKind {
    match s {
        "depends_on" => RelationKind::DependsOn,
        "extends" => RelationKind::Extends,
        _ => RelationKind::RelatesTo,
    }
}

fn collect_section_text(sections: &[Section], out: &mut String) {
    for s in sections {
        out.push_str(&s.title);
        out.push('\n');
        out.push_str(&s.body_md);
        out.push('\n');
        collect_section_text(&s.children, out);
    }
}

/// Remove a plan and everything derived from it. Returns whether it existed.
fn delete_plan(conn: &Connection, id: &str) -> Result<bool> {
    let rowids: Vec<i64> = conn
        .prepare("SELECT fts_rowid FROM fts_rows WHERE plan_id = ?1")?
        .query_map([id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for rowid in rowids {
        conn.execute("DELETE FROM search_fts WHERE rowid = ?1", [rowid])?;
    }
    // tasks, tags, relations and fts_rows go with it via ON DELETE CASCADE
    Ok(conn.execute("DELETE FROM plans WHERE id = ?1", [id])? > 0)
}

fn record_fts_row(conn: &Connection, plan_id: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO fts_rows (plan_id, fts_rowid) VALUES (?1, ?2)",
        params![plan_id, conn.last_insert_rowid()],
    )?;
    Ok(())
}

const SUMMARY_SQL: &str =
    "SELECT p.id, p.project, p.title, p.created, p.structured, p.warnings_count,
        (SELECT count(*) FROM tasks t WHERE t.plan_id = p.id),
        (SELECT count(*) FROM tasks t WHERE t.plan_id = p.id AND t.status = 'done')
     FROM plans p";

fn query_summaries(
    conn: &Connection,
    clause: &str,
    args: &[&dyn ToSql],
) -> Result<Vec<PlanSummary>> {
    let sql = format!("{SUMMARY_SQL} {clause} ORDER BY p.id");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows: Vec<PlanSummary> = stmt
        .query_map(args, |r| {
            Ok(PlanSummary {
                id: r.get(0)?,
                project: r.get(1)?,
                title: r.get(2)?,
                created: r.get(3)?,
                structured: r.get(4)?,
                warnings_count: r.get(5)?,
                task_count: r.get(6)?,
                done_count: r.get(7)?,
                tags: vec![],
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    let mut stmt = conn.prepare("SELECT plan_id, tag FROM plan_tags ORDER BY rowid")?;
    for r in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (id, tag) = r?;
        tags.entry(id).or_default().push(tag);
    }
    for row in &mut rows {
        row.tags = tags.remove(&row.id).unwrap_or_default();
    }
    Ok(rows)
}

fn task_rows(conn: &Connection, clause: &str, args: &[&dyn ToSql]) -> Result<Vec<TaskRow>> {
    let sql = format!(
        "SELECT t.plan_id, p.title, p.project, t.json
         FROM tasks t JOIN plans p ON p.id = t.plan_id {clause}
         ORDER BY t.plan_id, t.pos"
    );
    let mut stmt = conn.prepare(&sql)?;
    let raw: Vec<(String, String, String, String)> = stmt
        .query_map(args, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?;
    raw.into_iter()
        .map(|(plan_id, plan_title, project, json)| {
            Ok(TaskRow {
                plan_id,
                plan_title,
                project,
                task: serde_json::from_str::<Task>(&json)?,
            })
        })
        .collect()
}

impl Store {
    /// Insert or replace a plan. No-op when id, hash and path are unchanged.
    pub fn upsert_plan(&self, plan: &Plan) -> Result<()> {
        let conn = self.conn();
        let path = path_str(&plan.source_path);
        let existing: Option<(String, String)> = conn
            .query_row(
                "SELECT hash, path FROM plans WHERE id = ?1",
                [&plan.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if existing
            .as_ref()
            .is_some_and(|(h, p)| *h == plan.content_hash && *p == path)
        {
            return Ok(());
        }

        let tx = conn.unchecked_transaction()?;
        // a different plan previously at this path (e.g. id edited in place)
        let stale: Option<String> = tx
            .query_row("SELECT id FROM plans WHERE path = ?1", [&path], |r| {
                r.get(0)
            })
            .optional()?;
        if let Some(stale) = stale {
            delete_plan(&tx, &stale)?;
        }
        if existing.is_some() {
            delete_plan(&tx, &plan.id)?;
        }

        tx.execute(
            "INSERT INTO plans (id, project, title, path, json, hash, created, structured, warnings_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                plan.id,
                plan.project,
                plan.title,
                path,
                serde_json::to_string(plan)?,
                plan.content_hash,
                plan.created,
                plan.structured,
                plan.warnings.len() as i64,
            ],
        )?;
        for tag in &plan.tags {
            tx.execute(
                "INSERT OR IGNORE INTO plan_tags (plan_id, tag) VALUES (?1, ?2)",
                [&plan.id, tag],
            )?;
        }
        for rel in &plan.related {
            tx.execute(
                "INSERT OR IGNORE INTO relations (from_id, to_id, kind) VALUES (?1, ?2, ?3)",
                params![plan.id, rel.to, kind_str(rel.kind)],
            )?;
        }

        let mut fts = String::new();
        collect_section_text(&plan.sections, &mut fts);
        let all_tags = plan
            .tags
            .iter()
            .chain(plan.tasks.iter().flat_map(|t| &t.tags));
        let body = format!("{fts}\n{}\n{}", plan.project, self.tag_labels(all_tags));
        tx.execute(
            "INSERT INTO search_fts (kind, ref, title, body) VALUES ('plan', ?1, ?2, ?3)",
            params![plan.id, super::fold(&plan.title), super::fold(&body)],
        )?;
        record_fts_row(&tx, &plan.id)?;

        for (pos, task) in plan.tasks.iter().enumerate() {
            tx.execute(
                "INSERT OR REPLACE INTO tasks (plan_id, id, pos, name, status, complexity, json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    plan.id,
                    task.id,
                    pos as i64,
                    task.name,
                    serde_json::to_value(task.status)?
                        .as_str()
                        .unwrap_or("todo"),
                    task.complexity,
                    serde_json::to_string(task)?,
                ],
            )?;
            for tag in &task.tags {
                tx.execute(
                    "INSERT OR IGNORE INTO task_tags (plan_id, task_id, tag) VALUES (?1, ?2, ?3)",
                    [&plan.id, &task.id, tag],
                )?;
            }
            let body = format!("{}\n{}", task.body_md, self.tag_labels(task.tags.iter()));
            tx.execute(
                "INSERT INTO search_fts (kind, ref, title, body) VALUES ('task', ?1, ?2, ?3)",
                params![
                    format!("{}{REF_SEP}{}", plan.id, task.id),
                    super::fold(&task.name),
                    super::fold(&body)
                ],
            )?;
            record_fts_row(&tx, &plan.id)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_by_path(&self, path: &Path) -> Result<()> {
        let conn = self.conn();
        let path = path_str(path);
        let id: Option<String> = conn
            .query_row("SELECT id FROM plans WHERE path = ?1", [&path], |r| {
                r.get(0)
            })
            .optional()?;
        if let Some(id) = id {
            let tx = conn.unchecked_transaction()?;
            delete_plan(&tx, &id)?;
            tx.commit()?;
        }
        Ok(())
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectSummary>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT p.project, count(DISTINCT p.id), count(t.id),
                    coalesce(sum(t.status = 'done'), 0)
             FROM plans p LEFT JOIN tasks t ON t.plan_id = p.id
             GROUP BY p.project ORDER BY p.project",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ProjectSummary {
                    name: r.get(0)?,
                    plan_count: r.get(1)?,
                    task_count: r.get(2)?,
                    done_count: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    pub fn list_plans(&self, f: &PlanFilter) -> Result<Vec<PlanSummary>> {
        query_summaries(
            &self.conn(),
            "WHERE (?1 IS NULL OR p.project = ?1)
               AND (?2 IS NULL
                    OR EXISTS (SELECT 1 FROM plan_tags g WHERE g.plan_id = p.id AND g.tag = ?2)
                    OR EXISTS (SELECT 1 FROM task_tags g WHERE g.plan_id = p.id AND g.tag = ?2))",
            &[&f.project, &f.tag],
        )
    }

    pub(super) fn plan_summary(&self, id: &str) -> Result<Option<PlanSummary>> {
        Ok(query_summaries(&self.conn(), "WHERE p.id = ?1", &[&id])?
            .into_iter()
            .next())
    }

    pub(super) fn task_row(&self, plan_id: &str, task_id: &str) -> Result<Option<TaskRow>> {
        Ok(task_rows(
            &self.conn(),
            "WHERE t.plan_id = ?1 AND t.id = ?2",
            &[&plan_id, &task_id],
        )?
        .into_iter()
        .next())
    }

    pub fn get_plan(&self, id: &str) -> Result<Option<Plan>> {
        let json: Option<String> = self
            .conn()
            .query_row("SELECT json FROM plans WHERE id = ?1", [id], |r| r.get(0))
            .optional()?;
        Ok(json.map(|j| serde_json::from_str(&j)).transpose()?)
    }

    pub fn list_tasks(&self, f: &TaskFilter) -> Result<Vec<TaskRow>> {
        task_rows(
            &self.conn(),
            "WHERE (?1 IS NULL OR p.project = ?1)
               AND (?2 IS NULL OR EXISTS (SELECT 1 FROM task_tags g
                    WHERE g.plan_id = t.plan_id AND g.task_id = t.id AND g.tag = ?2))",
            &[&f.project, &f.tag],
        )
    }

    /// Per tag: number of plans using it (directly or via a task) and number of tasks.
    pub fn topics(&self) -> Result<Vec<TopicCount>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "WITH used(tag, plan_id, task_id) AS (
                 SELECT tag, plan_id, NULL FROM plan_tags
                 UNION ALL SELECT tag, plan_id, task_id FROM task_tags)
             SELECT tag, count(DISTINCT plan_id), count(task_id) FROM used
             GROUP BY tag ORDER BY tag",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(TopicCount {
                    tag: r.get(0)?,
                    plans: r.get(1)?,
                    tasks: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    pub fn graph(&self) -> Result<Graph> {
        let conn = self.conn();
        let nodes = query_summaries(&conn, "", &[])?;
        let ids: BTreeSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

        let mut edges = Vec::new();
        let mut connected: BTreeSet<(String, String)> = BTreeSet::new();
        let mut stmt = conn
            .prepare("SELECT from_id, to_id, kind FROM relations ORDER BY from_id, to_id, kind")?;
        for r in stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (from, to, kind) = r?;
            if !ids.contains(to.as_str()) {
                continue;
            }
            connected.insert((from.clone(), to.clone()));
            connected.insert((to.clone(), from.clone()));
            edges.push(Edge {
                from,
                to,
                kind: EdgeKind::Declared(kind_from_str(&kind)),
            });
        }

        let topical: Vec<(&str, BTreeSet<&str>)> = nodes
            .iter()
            .map(|n| {
                let tags = n
                    .tags
                    .iter()
                    .map(String::as_str)
                    .filter(|t| t.starts_with("domain/") || t.starts_with("tech/"))
                    .collect();
                (n.id.as_str(), tags)
            })
            .collect();
        let mut shared_edges: BTreeMap<(usize, usize), Vec<String>> = BTreeMap::new();
        for (i, (a, ta)) in topical.iter().enumerate() {
            for (j, (b, tb)) in topical.iter().enumerate().skip(i + 1) {
                if connected.contains(&(a.to_string(), b.to_string())) {
                    continue;
                }
                let shared: Vec<String> = ta.intersection(tb).map(|s| s.to_string()).collect();
                if !shared.is_empty() {
                    shared_edges.insert((i, j), shared);
                }
            }
        }
        for ((i, j), shared) in shared_edges {
            edges.push(Edge {
                from: topical[i].0.to_string(),
                to: topical[j].0.to_string(),
                kind: EdgeKind::SharedTopics(shared),
            });
        }
        Ok(Graph { nodes, edges })
    }

    pub fn plan_ids_for_project(&self, project: &str) -> Result<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id FROM plans WHERE project = ?1 ORDER BY id")?;
        Ok(stmt
            .query_map([project], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    /// Source file of a plan.
    pub fn find_task_path(&self, plan_id: &str) -> Result<Option<PathBuf>> {
        let p: Option<String> = self
            .conn()
            .query_row("SELECT path FROM plans WHERE id = ?1", [plan_id], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(p.map(PathBuf::from))
    }
}
