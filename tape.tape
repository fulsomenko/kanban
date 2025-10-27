# VHS recording demonstrating kanban application capabilities
# Showcases: board creation, columns, cards, sprints, editing, and navigation

Output tape.gif

Require echo

Set Shell "zsh"
Set FontSize 16
Set Width 1920
Set Height 1080
Set Padding 20
Set Margin 10
Set MarginFill "#2e2e2e"
Set BorderRadius 8
Set WindowBar "Colorful"

# Start the application
Type "cargo run --release" Enter
Sleep 3s

# ========== CREATE BOARD ==========
Type "n" Sleep 500ms
Type "Web Platform MVP" Enter
Sleep 1s

# ========== CREATE COLUMNS ==========
# Column 1: Todo
Type "n" Sleep 500ms
Type "Todo" Enter
Sleep 500ms

# Column 2: In Progress
Type "n" Sleep 500ms
Type "In Progress" Enter
Sleep 500ms

# Column 3: Done
Type "n" Sleep 500ms
Type "Done" Enter
Sleep 1s

# ========== CREATE CARDS IN TODO COLUMN ==========
# Move to first card position
Enter

# Card 1: High priority design task
Type "n" Sleep 500ms
Type "Design landing page" Enter
Sleep 1s

# Edit card details (add points)
Type "e" Sleep 500ms
Down Down Down
Type "3" Enter
Sleep 500ms
Escape Sleep 500ms

# Card 2: Setup infrastructure
Type "n" Sleep 500ms
Type "Setup CI/CD pipeline" Enter
Sleep 1s

Type "e" Sleep 500ms
Down Down Down
Type "5" Enter
Sleep 500ms
Escape Sleep 500ms

# Card 3: Database schema
Type "n" Sleep 500ms
Type "Design database schema" Enter
Sleep 1s

Type "e" Sleep 500ms
Down Down Down
Type "3" Enter
Sleep 500ms
Escape Sleep 500ms

# ========== MOVE CARDS BETWEEN COLUMNS ==========
# Navigate to second column (In Progress)
Right Sleep 500ms

# Add card to In Progress
Type "n" Sleep 500ms
Type "Implement authentication" Enter
Sleep 1s

Type "e" Sleep 500ms
Down Down Down
Type "5" Enter
Sleep 500ms
Escape Sleep 500ms

# ========== MOVE TO DONE COLUMN ==========
Right Sleep 500ms

# Add completed card
Type "n" Sleep 500ms
Type "Project kickoff meeting" Enter
Sleep 1s

Type "e" Sleep 500ms
Down Down Down
Type "2" Enter
Sleep 500ms
Escape Sleep 500ms

# ========== VIEW BOARD DETAILS & SPRINTS ==========
# Navigate back to board view
Home Sleep 500ms

# Open board details
Type "e" Sleep 1s

# View sprints panel
Right Right Right Sleep 500ms

# Create a new sprint
Type "s" Sleep 500ms
Type "1" Enter  # Sprint duration
Sleep 1s

# Return to main view
Escape Sleep 500ms

# ========== MULTI-SELECT AND ASSIGN TO SPRINT ==========
# Select first card
Type "v" Sleep 500ms

# Move down and select more cards
Down Type "v" Sleep 500ms
Down Type "v" Sleep 500ms

# Assign selected cards to sprint
Type "a" Sleep 1s
Type "1" Enter
Sleep 1s

# ========== TOGGLE COMPLETION ==========
# Toggle completion on selected cards
Type "c" Sleep 1s

# ========== SPRINT FILTER ==========
# Toggle sprint filter to show only active sprint tasks
Type "t" Sleep 1s

# ========== NAVIGATION DEMO ==========
# Navigate to card detail view
Enter Sleep 2s

# Show different panels
Type "1" Sleep 500ms  # Title panel
Sleep 500ms
Type "2" Sleep 500ms  # Metadata panel
Sleep 500ms
Type "3" Sleep 500ms  # Description panel
Sleep 1s

# Return to main view
Escape Sleep 500ms

# ========== FINAL PAUSE ==========
Sleep 2s

# Show quit command
Type "q"
Sleep 500ms
