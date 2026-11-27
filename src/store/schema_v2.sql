-- at: unix ms; kind: pre_tool | subagent_start | subagent_stop | session_end.
-- plan_id/task_id: the session's task link when the event was recorded (NULL if unlinked).
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY,
    at INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    role TEXT NOT NULL,
    kind TEXT NOT NULL,
    tool TEXT,
    step_json TEXT,
    plan_id TEXT,
    task_id TEXT
);
CREATE INDEX IF NOT EXISTS events_task ON events(plan_id, task_id, id);
CREATE INDEX IF NOT EXISTS events_at ON events(at);

CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
