CREATE TABLE plans (
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
CREATE INDEX plans_project ON plans(project);

CREATE TABLE tasks (
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    pos INTEGER NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    complexity INTEGER,
    json TEXT NOT NULL,
    PRIMARY KEY (plan_id, id)
);

CREATE TABLE plan_tags (
    plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (plan_id, tag)
);
CREATE INDEX plan_tags_tag ON plan_tags(tag);

CREATE TABLE task_tags (
    plan_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (plan_id, task_id, tag),
    FOREIGN KEY (plan_id, task_id) REFERENCES tasks(plan_id, id) ON DELETE CASCADE
);
CREATE INDEX task_tags_tag ON task_tags(tag);

CREATE TABLE relations (
    from_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    to_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    PRIMARY KEY (from_id, to_id, kind)
);

-- kind: 'plan' (ref = plan id) or 'task' (ref = plan id + US + task id)
CREATE VIRTUAL TABLE search_fts USING fts5(
    kind UNINDEXED,
    ref UNINDEXED,
    title,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- bumped on every write; lets callers detect index changes cheaply
CREATE TABLE meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
INSERT INTO meta (key, value) VALUES ('data_version', 0);
