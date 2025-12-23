-- SQLite schema for kanban persistence
-- Version: 1

-- Enable WAL mode for better concurrent read performance
PRAGMA journal_mode=WAL;

-- Metadata table for tracking persistence state and conflict detection
CREATE TABLE IF NOT EXISTS metadata (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Singleton row
    instance_id TEXT NOT NULL,
    saved_at TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1
);

-- Boards table
CREATE TABLE IF NOT EXISTS boards (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    sprint_prefix TEXT,
    card_prefix TEXT,
    task_sort_field TEXT NOT NULL DEFAULT 'Default',
    task_sort_order TEXT NOT NULL DEFAULT 'Ascending',
    sprint_duration_days INTEGER,
    sprint_names TEXT NOT NULL DEFAULT '[]',  -- JSON array
    sprint_name_used_count INTEGER NOT NULL DEFAULT 0,
    next_sprint_number INTEGER NOT NULL DEFAULT 1,
    active_sprint_id TEXT,
    task_list_view TEXT NOT NULL DEFAULT 'Flat',
    prefix_counters TEXT NOT NULL DEFAULT '{}',  -- JSON object
    sprint_counters TEXT NOT NULL DEFAULT '{}',  -- JSON object
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Columns table
CREATE TABLE IF NOT EXISTS columns (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    wip_limit INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
);

-- Sprints table (defined before cards since cards reference sprints)
CREATE TABLE IF NOT EXISTS sprints (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL,
    sprint_number INTEGER NOT NULL,
    name_index INTEGER,
    prefix TEXT,
    card_prefix TEXT,
    status TEXT NOT NULL DEFAULT 'Planning',
    start_date TEXT,
    end_date TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
);

-- Cards table
CREATE TABLE IF NOT EXISTS cards (
    id TEXT PRIMARY KEY,
    column_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    priority TEXT NOT NULL DEFAULT 'Medium',
    status TEXT NOT NULL DEFAULT 'Todo',
    position INTEGER NOT NULL,
    due_date TEXT,
    points INTEGER,
    card_number INTEGER NOT NULL DEFAULT 0,
    sprint_id TEXT,
    assigned_prefix TEXT,
    card_prefix TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    sprint_logs TEXT NOT NULL DEFAULT '[]',  -- JSON array
    FOREIGN KEY (column_id) REFERENCES columns(id) ON DELETE CASCADE,
    FOREIGN KEY (sprint_id) REFERENCES sprints(id) ON DELETE SET NULL
);

-- Archived cards table
CREATE TABLE IF NOT EXISTS archived_cards (
    id TEXT PRIMARY KEY,  -- Same as embedded card.id
    card_data TEXT NOT NULL,  -- JSON blob of Card
    archived_at TEXT NOT NULL,
    original_column_id TEXT NOT NULL,
    original_position INTEGER NOT NULL
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_columns_board_id ON columns(board_id);
CREATE INDEX IF NOT EXISTS idx_columns_position ON columns(board_id, position);

CREATE INDEX IF NOT EXISTS idx_sprints_board_id ON sprints(board_id);
CREATE INDEX IF NOT EXISTS idx_sprints_status ON sprints(status);

CREATE INDEX IF NOT EXISTS idx_cards_column_id ON cards(column_id);
CREATE INDEX IF NOT EXISTS idx_cards_sprint_id ON cards(sprint_id);
CREATE INDEX IF NOT EXISTS idx_cards_position ON cards(column_id, position);
CREATE INDEX IF NOT EXISTS idx_cards_status ON cards(status);
CREATE INDEX IF NOT EXISTS idx_cards_priority ON cards(priority);
CREATE INDEX IF NOT EXISTS idx_cards_updated_at ON cards(updated_at);

CREATE INDEX IF NOT EXISTS idx_archived_cards_archived_at ON archived_cards(archived_at);
