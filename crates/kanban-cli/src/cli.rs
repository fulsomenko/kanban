use clap::{Args, Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "kanban")]
#[command(about = "A terminal-based kanban board", long_about = None)]
#[command(version, arg_required_else_help = false)]
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
    List,
    /// Get a specific board
    Get {
        #[arg(long)]
        id: Uuid,
    },
    /// Update a board
    Update(BoardUpdateArgs),
    /// Delete a board
    Delete {
        #[arg(long)]
        id: Uuid,
    },
}

#[derive(Args)]
pub struct BoardUpdateArgs {
    #[arg(long)]
    pub id: Uuid,
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
        #[arg(long)]
        board_id: Uuid,
        #[arg(long)]
        name: String,
        #[arg(long)]
        position: Option<i32>,
    },
    /// List columns for a board
    List {
        #[arg(long)]
        board_id: Uuid,
    },
    /// Get a specific column
    Get {
        #[arg(long)]
        id: Uuid,
    },
    /// Update a column
    Update(ColumnUpdateArgs),
    /// Delete a column
    Delete {
        #[arg(long)]
        id: Uuid,
    },
    /// Reorder a column
    Reorder {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        position: i32,
    },
}

#[derive(Args)]
pub struct ColumnUpdateArgs {
    #[arg(long)]
    pub id: Uuid,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub position: Option<i32>,
    #[arg(long)]
    pub wip_limit: Option<u32>,
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
    /// Get a specific card
    Get {
        #[arg(long)]
        id: Uuid,
    },
    /// Update a card
    Update(CardUpdateArgs),
    /// Move a card to another column
    Move {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        column_id: Uuid,
        #[arg(long)]
        position: Option<i32>,
    },
    /// Archive a card
    Archive {
        #[arg(long)]
        id: Uuid,
    },
    /// Restore an archived card
    Restore {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        column_id: Option<Uuid>,
    },
    /// Permanently delete an archived card
    Delete {
        #[arg(long)]
        id: Uuid,
    },
    /// Assign a card to a sprint
    AssignSprint {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        sprint_id: Uuid,
    },
    /// Unassign a card from its sprint
    UnassignSprint {
        #[arg(long)]
        id: Uuid,
    },
    /// Get the branch name for a card
    BranchName {
        #[arg(long)]
        id: Uuid,
    },
    /// Get the git checkout command for a card
    GitCheckout {
        #[arg(long)]
        id: Uuid,
    },
    /// Archive multiple cards
    BulkArchive {
        #[arg(long, value_delimiter = ',')]
        ids: Vec<Uuid>,
    },
    /// Move multiple cards to a column
    BulkMove {
        #[arg(long, value_delimiter = ',')]
        ids: Vec<Uuid>,
        #[arg(long)]
        column_id: Uuid,
    },
    /// Assign multiple cards to a sprint
    BulkAssignSprint {
        #[arg(long, value_delimiter = ',')]
        ids: Vec<Uuid>,
        #[arg(long)]
        sprint_id: Uuid,
    },
}

#[derive(Args)]
pub struct CardCreateArgs {
    #[arg(long)]
    pub board_id: Uuid,
    #[arg(long)]
    pub column_id: Uuid,
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
    #[arg(long)]
    pub board_id: Option<Uuid>,
    #[arg(long)]
    pub column_id: Option<Uuid>,
    #[arg(long)]
    pub sprint_id: Option<Uuid>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub archived: bool,
}

#[derive(Args)]
pub struct CardUpdateArgs {
    #[arg(long)]
    pub id: Uuid,
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
        #[arg(long)]
        board_id: Uuid,
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    /// List sprints for a board
    List {
        #[arg(long)]
        board_id: Uuid,
    },
    /// Get a specific sprint
    Get {
        #[arg(long)]
        id: Uuid,
    },
    /// Update a sprint
    Update(SprintUpdateArgs),
    /// Activate a sprint
    Activate {
        #[arg(long)]
        id: Uuid,
        #[arg(long)]
        duration_days: Option<i32>,
    },
    /// Complete a sprint
    Complete {
        #[arg(long)]
        id: Uuid,
    },
    /// Cancel a sprint
    Cancel {
        #[arg(long)]
        id: Uuid,
    },
    /// Delete a sprint
    Delete {
        #[arg(long)]
        id: Uuid,
    },
}

#[derive(Args)]
pub struct SprintUpdateArgs {
    #[arg(long)]
    pub id: Uuid,
    #[arg(long)]
    pub prefix: Option<String>,
    #[arg(long)]
    pub card_prefix: Option<String>,
}

// Export/Import commands
#[derive(Args)]
pub struct ExportArgs {
    #[arg(long)]
    pub board_id: Option<Uuid>,
}

#[derive(Args)]
pub struct ImportArgs {
    #[arg(long)]
    pub file: String,
}
