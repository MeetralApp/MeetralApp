PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meeting_folder (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  parent_id TEXT REFERENCES meeting_folder(id) ON DELETE SET NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS meeting_record (
  id TEXT PRIMARY KEY NOT NULL,
  folder_id TEXT REFERENCES meeting_folder(id) ON DELETE SET NULL,
  title TEXT NOT NULL,
  my_language TEXT NOT NULL DEFAULT '',
  meeting_language TEXT NOT NULL DEFAULT '',
  session_mode TEXT NOT NULL DEFAULT 'interpreter',
  started_at_ms INTEGER NOT NULL,
  ended_at_ms INTEGER,
  status TEXT NOT NULL CHECK (status IN ('live', 'ended'))
);

CREATE INDEX IF NOT EXISTS idx_meeting_record_folder ON meeting_record(folder_id);
CREATE INDEX IF NOT EXISTS idx_meeting_record_started ON meeting_record(started_at_ms DESC);

CREATE TABLE IF NOT EXISTS transcript_segment (
  id TEXT PRIMARY KEY NOT NULL,
  meeting_id TEXT NOT NULL REFERENCES meeting_record(id) ON DELETE CASCADE,
  direction TEXT NOT NULL CHECK (direction IN ('outbound', 'inbound')),
  sequence INTEGER NOT NULL,
  source_text TEXT NOT NULL DEFAULT '',
  translated_text TEXT NOT NULL DEFAULT '',
  started_at_ms INTEGER NOT NULL,
  ended_at_ms INTEGER NOT NULL,
  connection_gap INTEGER NOT NULL DEFAULT 0,
  UNIQUE (meeting_id, direction, sequence)
);

CREATE INDEX IF NOT EXISTS idx_segment_meeting_dir_seq ON transcript_segment(meeting_id, direction, sequence);

CREATE VIRTUAL TABLE IF NOT EXISTS transcript_fts USING fts5(
  segment_id UNINDEXED,
  meeting_id UNINDEXED,
  source_text,
  translated_text,
  tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS transcript_segment_ad_fts
AFTER DELETE ON transcript_segment BEGIN
  DELETE FROM transcript_fts WHERE segment_id = old.id;
END;

CREATE TABLE IF NOT EXISTS meeting_summary (
  meeting_id TEXT PRIMARY KEY NOT NULL REFERENCES meeting_record(id) ON DELETE CASCADE,
  template_id TEXT NOT NULL DEFAULT 'meeting_brief',
  summary_language TEXT NOT NULL DEFAULT '',
  generated_json TEXT NOT NULL DEFAULT '{}',
  generated_at_ms INTEGER
);

CREATE TABLE IF NOT EXISTS audio_chunk (
  id TEXT PRIMARY KEY NOT NULL,
  meeting_id TEXT NOT NULL REFERENCES meeting_record(id) ON DELETE CASCADE,
  direction TEXT NOT NULL CHECK (direction IN ('outbound', 'inbound')),
  sequence INTEGER NOT NULL,
  file_path TEXT NOT NULL,
  duration_ms INTEGER NOT NULL,
  sample_rate INTEGER NOT NULL DEFAULT 16000,
  started_at_ms INTEGER NOT NULL,
  byte_size INTEGER NOT NULL DEFAULT 0,
  UNIQUE (meeting_id, direction, sequence)
);

CREATE TABLE IF NOT EXISTS meeting_artifact (
  id            TEXT PRIMARY KEY,
  meeting_id    TEXT NOT NULL REFERENCES meeting_record(id) ON DELETE CASCADE,
  topic_id      TEXT,
  kind          TEXT NOT NULL,
  text          TEXT NOT NULL,
  owner         TEXT,
  due           TEXT,
  status        TEXT NOT NULL DEFAULT 'proposed',
  supersedes_artifact_id TEXT REFERENCES meeting_artifact(id),
  segment_refs  TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  origin        TEXT NOT NULL DEFAULT 'extracted'
);
CREATE INDEX IF NOT EXISTS idx_artifact_query
  ON meeting_artifact(meeting_id, kind, status);
CREATE INDEX IF NOT EXISTS idx_artifact_supersedes
  ON meeting_artifact(supersedes_artifact_id);

CREATE TABLE IF NOT EXISTS meeting_entity (
  id            TEXT PRIMARY KEY,
  meeting_id    TEXT NOT NULL REFERENCES meeting_record(id) ON DELETE CASCADE,
  topic_id      TEXT,
  name          TEXT NOT NULL,
  kind          TEXT NOT NULL DEFAULT 'generic',
  segment_refs  TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  origin        TEXT NOT NULL DEFAULT 'extracted',
  UNIQUE(meeting_id, name)
);

CREATE VIRTUAL TABLE IF NOT EXISTS entity_fts USING fts5(
  entity_id  UNINDEXED,
  meeting_id UNINDEXED,
  name,
  tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS meeting_entity_ad_fts
AFTER DELETE ON meeting_entity BEGIN
  DELETE FROM entity_fts WHERE entity_id = old.id;
END;
