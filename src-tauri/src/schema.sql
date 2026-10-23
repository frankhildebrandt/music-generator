PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS songs (
    id TEXT PRIMARY KEY, group_id TEXT NOT NULL, created_at TEXT NOT NULL,
    finished_at TEXT, status TEXT NOT NULL, message TEXT, title TEXT,
    caption TEXT NOT NULL, lyrics TEXT, seed INTEGER NOT NULL, params TEXT NOT NULL,
    result_meta TEXT, engine TEXT, file TEXT, favorite INTEGER NOT NULL DEFAULT 0,
    folder_id TEXT, started_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_songs_status ON songs(status);
CREATE INDEX IF NOT EXISTS idx_songs_group ON songs(group_id);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS folders (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS pro_projects (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, document TEXT NOT NULL,
    undo TEXT NOT NULL DEFAULT '[]', redo TEXT NOT NULL DEFAULT '[]', updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS pro_takes (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, document TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_pro_takes_project ON pro_takes(project_id);
CREATE TABLE IF NOT EXISTS pro_jobs (
    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, status TEXT NOT NULL, message TEXT NOT NULL,
    take_id TEXT, request TEXT NOT NULL, snapshot TEXT NOT NULL, source_revision INTEGER NOT NULL, created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_pro_jobs_status ON pro_jobs(status);
