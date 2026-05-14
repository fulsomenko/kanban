use clap::{Args, Parser, Subcommand};

use kanban_core::VERSION;

#[derive(Parser)]
#[command(name = "kanban")]
#[command(about = "A terminal-based kanban board", long_about = None)]
#[command(version = VERSION, arg_required_else_help = false)]
pub struct Cli {
    /// Path to kanban data file (or set KANBAN_FILE env var)
    #[arg(value_name = "FILE", env = "KANBAN_FILE")]
    pub file: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Board operations
    Board(BoardCommand),
    /// Column operations
    Column(ColumnCommand),
    /// Card operations
    Card(CardCommand),
    /// Sprint operations
    Sprint(SprintCommand),
    /// Export board data
    Export(ExportArgs),
    /// Import board data
    Import(ImportArgs),
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Migrate data between storage backends
    Migrate(MigrateArgs),
}

// Board commands
#[derive(Args)]
pub struct BoardCommand {
    #[command(subcommand)]
    pub action: BoardAction,
}

#[derive(Subcommand)]
pub enum BoardAction {
    /// Create a new board
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        card_prefix: Option<String>,
    },
    /// List all boards
    List {
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Get a specific board by UUID or name
    Get {
        /// Board UUID or name
        id: String,
    },
    /// Update a board
    Update(BoardUpdateArgs),
    /// Delete a board by UUID or name
    Delete {
        /// Board UUID or name
        id: String,
    },
}

#[derive(Args)]
pub struct BoardUpdateArgs {
    /// Board UUID or name
    pub id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub sprint_prefix: Option<String>,
    #[arg(long)]
    pub card_prefix: Option<String>,
}

// Column commands
#[derive(Args)]
pub struct ColumnCommand {
    #[command(subcommand)]
    pub action: ColumnAction,
}

#[derive(Subcommand)]
pub enum ColumnAction {
    /// Create a new column
    Create {
        /// Board UUID or name
        #[arg(long)]
        board_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        position: Option<i32>,
    },
    /// List columns for a board
    List {
        /// Board UUID or name
        #[arg(long)]
        board_id: String,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Get a specific column by UUID or name
    Get {
        /// Column UUID or name
        id: String,
    },
    /// Update a column
    Update(ColumnUpdateArgs),
    /// Delete a column by UUID or name
    Delete {
        /// Column UUID or name
        id: String,
    },
    /// Reorder a column by UUID or name
    Reorder {
        /// Column UUID or name
        id: String,
        #[arg(long)]
        position: i32,
    },
}

#[derive(Args)]
pub struct ColumnUpdateArgs {
    /// Column UUID or name
    pub id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub position: Option<i32>,
    #[arg(long)]
    pub wip_limit: Option<u32>,
    #[arg(long)]
    pub clear_wip_limit: bool,
}

// Card commands
#[derive(Args)]
pub struct CardCommand {
    #[command(subcommand)]
    pub action: CardAction,
}

#[derive(Subcommand)]
pub enum CardAction {
    /// Create a new card
    Create(CardCreateArgs),
    /// List cards with optional filters
    List(CardListArgs),
    /// Get a specific card by ID or identifier (e.g. KAN-5)
    Get {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
    },
    /// Update a card
    Update(CardUpdateArgs),
    /// Move a card to another column
    Move {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
        /// Column UUID or name
        #[arg(long)]
        column_id: String,
        #[arg(long)]
        position: Option<i32>,
    },
    /// Archive a card by ID or identifier (e.g. KAN-5)
    Archive {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
    },
    /// Restore an archived card by ID or identifier (e.g. KAN-5)
    Restore {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
        /// Column UUID or name
        #[arg(long)]
        column_id: Option<String>,
    },
    /// Permanently delete an archived card by ID or identifier (e.g. KAN-5)
    Delete {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
    },
    /// Assign a card to a sprint
    AssignSprint {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
        /// Sprint UUID, name, or number
        #[arg(long)]
        sprint_id: String,
    },
    /// Unassign a card from its sprint
    UnassignSprint {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
    },
    /// Get the branch name for a card
    BranchName {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
    },
    /// Get the git checkout command for a card
    GitCheckout {
        /// Card UUID or identifier like KAN-5 or 5
        id: String,
    },
    /// Archive multiple cards
    #[command(name = "archive-cards")]
    ArchiveCards {
        /// Comma-separated card UUIDs or identifiers (e.g. KAN-1,KAN-2,42)
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
    },
    /// Move multiple cards to a column
    #[command(name = "move-cards")]
    MoveCards {
        /// Comma-separated card UUIDs or identifiers (e.g. KAN-1,KAN-2,42)
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
        /// Column UUID or name (must be on the same board as all selected cards)
        #[arg(long)]
        column_id: String,
    },
    /// Assign multiple cards to a sprint
    #[command(name = "assign-cards-to-sprint")]
    AssignCardsToSprint {
        /// Comma-separated card UUIDs or identifiers (e.g. KAN-1,KAN-2,42)
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
        /// Sprint UUID, name, or number (must be on the same board as all selected cards)
        #[arg(long)]
        sprint_id: String,
    },
}

#[derive(Args)]
pub struct CardCreateArgs {
    /// Board UUID or name
    #[arg(long)]
    pub board_id: String,
    /// Column UUID or name
    #[arg(long)]
    pub column_id: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub priority: Option<String>,
    #[arg(long)]
    pub points: Option<u8>,
    #[arg(long)]
    pub due_date: Option<String>,
}

#[derive(Args)]
pub struct CardListArgs {
    /// Board UUID or name
    #[arg(long)]
    pub board_id: Option<String>,
    /// Column UUID or name (scoped to --board-id if given, else searched globally)
    #[arg(long)]
    pub column_id: Option<String>,
    /// Sprint UUID, name, or number (scoped to --board-id if given, else searched globally)
    #[arg(long)]
    pub sprint_id: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub archived: bool,
    #[arg(long)]
    pub page: Option<u32>,
    #[arg(long)]
    pub page_size: Option<u32>,
}

#[derive(Args)]
pub struct CardUpdateArgs {
    /// Card UUID or identifier like KAN-5 or 5
    pub id: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub priority: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub points: Option<u8>,
    #[arg(long)]
    pub due_date: Option<String>,
    #[arg(long)]
    pub clear_due_date: bool,
}

// Sprint commands
#[derive(Args)]
pub struct SprintCommand {
    #[command(subcommand)]
    pub action: SprintAction,
}

#[derive(Subcommand)]
pub enum SprintAction {
    /// Create a new sprint
    Create {
        /// Board UUID or name
        #[arg(long)]
        board_id: String,
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    /// List sprints for a board
    List {
        /// Board UUID or name
        #[arg(long)]
        board_id: String,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Get a specific sprint by UUID, name, or number
    Get {
        /// Sprint UUID, name, or number
        id: String,
    },
    /// Update a sprint
    Update(SprintUpdateArgs),
    /// Activate a sprint by UUID, name, or number
    Activate {
        /// Sprint UUID, name, or number
        id: String,
        #[arg(long)]
        duration_days: Option<i32>,
    },
    /// Complete a sprint by UUID, name, or number
    Complete {
        /// Sprint UUID, name, or number
        id: String,
    },
    /// Cancel a sprint by UUID, name, or number
    Cancel {
        /// Sprint UUID, name, or number
        id: String,
    },
    /// Delete a sprint by UUID, name, or number
    Delete {
        /// Sprint UUID, name, or number
        id: String,
    },
    /// Carry over uncompleted cards from a completed sprint to a planning sprint
    CarryOver {
        /// Source sprint UUID, name, or number (must be completed)
        #[arg(long)]
        from: String,
        /// Target sprint UUID, name, or number (must be in planning; on same board as source)
        #[arg(long)]
        to: String,
    },
}

#[derive(Args)]
pub struct SprintUpdateArgs {
    /// Sprint UUID, name, or number
    pub id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub prefix: Option<String>,
    #[arg(long)]
    pub card_prefix: Option<String>,
    #[arg(long)]
    pub start_date: Option<String>,
    #[arg(long)]
    pub end_date: Option<String>,
    #[arg(long)]
    pub clear_start_date: bool,
    #[arg(long)]
    pub clear_end_date: bool,
}

// Migrate command
#[derive(Args)]
#[command(after_help = "EXAMPLES:
    kanban migrate boards.json sqlite
    kanban migrate boards.json sqlite -o /path/to/output.sqlite
    kanban migrate boards.sqlite json -o boards.json
    kanban migrate data.bin json --source-backend sqlite")]
pub struct MigrateArgs {
    /// Path to source file
    pub source: String,
    /// Target backend name
    pub backend: String,
    /// Output path (default: derived from source filename and target backend)
    #[arg(long, short)]
    pub output: Option<String>,
    /// Override source backend auto-detection
    #[arg(long)]
    pub source_backend: Option<String>,
}

// Export/Import commands
#[derive(Args)]
pub struct ExportArgs {
    /// Board UUID or name; if omitted, exports all boards
    #[arg(long)]
    pub board_id: Option<String>,
}

#[derive(Args)]
pub struct ImportArgs {
    #[arg(long)]
    pub file: String,
}
