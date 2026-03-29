-- SQLite schema for kanban persistence
-- Version: 1

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
    sprint_name_used_count INTEGER NOT NULL DEFAULT 0,
    next_sprint_number INTEGER NOT NULL DEFAULT 1,
    active_sprint_id TEXT,
    task_list_view TEXT NOT NULL DEFAULT 'Flat',
    completion_column_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (active_sprint_id) REFERENCES sprints(id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (completion_column_id) REFERENCES columns(id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED
);

-- Board sprint names
CREATE TABLE IF NOT EXISTS board_sprint_names (
    board_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (board_id, position),
    FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
);

-- Board prefix counters
CREATE TABLE IF NOT EXISTS board_prefix_counters (
    board_id TEXT NOT NULL,
    prefix TEXT NOT NULL,
    counter INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (board_id, prefix),
    FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
);

-- Board sprint counters
CREATE TABLE IF NOT EXISTS board_sprint_counters (
    board_id TEXT NOT NULL,
    prefix TEXT NOT NULL,
    counter INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (board_id, prefix),
    FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
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

-- Cards table (holds both active and archived cards)
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
    FOREIGN KEY (column_id) REFERENCES columns(id) ON DELETE CASCADE,
    FOREIGN KEY (sprint_id) REFERENCES sprints(id) ON DELETE SET NULL
);

-- Sprint logs
-- Note: No FK on sprint_id — sprint logs are historical records
-- and must survive sprint deletion.
CREATE TABLE IF NOT EXISTS sprint_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id TEXT NOT NULL,
    sprint_id TEXT NOT NULL,
    sprint_number INTEGER NOT NULL,
    sprint_name TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    status TEXT NOT NULL,
    FOREIGN KEY (card_id) REFERENCES cards(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sprint_logs_card_id ON sprint_logs(card_id);

-- Archived cards metadata (card data lives in cards table)
CREATE TABLE IF NOT EXISTS archived_cards (
    card_id TEXT PRIMARY KEY,
    archived_at TEXT NOT NULL,
    original_column_id TEXT NOT NULL,
    original_position INTEGER NOT NULL,
    FOREIGN KEY (card_id) REFERENCES cards(id) ON DELETE CASCADE
);

-- Card dependency edges
CREATE TABLE IF NOT EXISTS card_edges (
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    direction TEXT NOT NULL,
    weight REAL,
    created_at TEXT NOT NULL,
    archived_at TEXT,
    PRIMARY KEY (source_id, target_id, edge_type)
);

CREATE INDEX IF NOT EXISTS idx_card_edges_source ON card_edges(source_id);
CREATE INDEX IF NOT EXISTS idx_card_edges_target ON card_edges(target_id);

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
