CREATE TABLE IF NOT EXISTS plans (
    id TEXT PRIMARY KEY,
    project TEXT NOT NULL,
    title TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    json TEXT NOT NULL,
    hash TEXT NOT NULL,
    created TEXT,
    structured INTEGER NOT NULL,
    warnings_count INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS plans_project ON plans(project);

CREATE TABLE IF NOT EXISTS tasks (
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    pos INTEGER NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    complexity INTEGER,
    json TEXT NOT NULL,
    PRIMARY KEY (plan_id, id)
);

CREATE TABLE IF NOT EXISTS plan_tags (
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (plan_id, tag)
);
CREATE INDEX IF NOT EXISTS plan_tags_tag ON plan_tags(tag);

CREATE TABLE IF NOT EXISTS task_tags (
    plan_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (plan_id, task_id, tag),
    FOREIGN KEY (plan_id, task_id) REFERENCES tasks(plan_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS task_tags_tag ON task_tags(tag);

CREATE TABLE IF NOT EXISTS relations (
    from_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    to_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    PRIMARY KEY (from_id, to_id, kind)
);

-- kind: 'plan' (ref = plan id) or 'task' (ref = plan id + US + task id)
CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
    kind UNINDEXED,
    ref UNINDEXED,
    title,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- rowids of a plan's search_fts rows, so they can be deleted without scanning the FTS table
CREATE TABLE IF NOT EXISTS fts_rows (
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    fts_rowid INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS fts_rows_plan ON fts_rows(plan_id);
