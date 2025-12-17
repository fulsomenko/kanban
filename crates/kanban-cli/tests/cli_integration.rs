use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn kanban() -> Command {
    Command::cargo_bin("kanban").unwrap()
}

fn parse_json_output(output: &str) -> Value {
    serde_json::from_str(output).expect("Failed to parse JSON output")
}

fn extract_id(json: &Value) -> String {
    json["data"]["id"].as_str().unwrap().to_string()
}

mod board_tests {
    use super::*;

    #[test]
    fn test_board_create() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["name"], "Test Board");
    }

    #[test]
    fn test_board_update_with_prefix() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let board_id = extract_id(&create_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "update",
                "--id",
                &board_id,
                "--card-prefix",
                "PROJ",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["card_prefix"], "PROJ");
    }

    #[test]
    fn test_board_list_empty() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        let output = kanban()
            .args(["--file", file.to_str().unwrap(), "board", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["count"], 0);
    }

    #[test]
    fn test_board_list_with_data() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Board 1",
            ])
            .assert()
            .success();

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Board 2",
            ])
            .assert()
            .success();

        let output = kanban()
            .args(["--file", file.to_str().unwrap(), "board", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["count"], 2);
    }

    #[test]
    fn test_board_get() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let board_id = extract_id(&create_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "get",
                "--id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["name"], "Test Board");
    }

    #[test]
    fn test_board_update() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Original",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let board_id = extract_id(&create_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "update",
                "--id",
                &board_id,
                "--name",
                "Updated",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["name"], "Updated");
    }

    #[test]
    fn test_board_delete() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "To Delete",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let board_id = extract_id(&create_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "delete",
                "--id",
                &board_id,
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"deleted\""));

        let list_output = kanban()
            .args(["--file", file.to_str().unwrap(), "board", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&list_output));
        assert_eq!(json["data"]["count"], 0);
    }
}

mod column_tests {
    use super::*;

    fn setup_board(file: &std::path::Path) -> String {
        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        extract_id(&json)
    }

    #[test]
    fn test_column_create() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "TODO",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["name"], "TODO");
        assert_eq!(json["data"]["board_id"], board_id);
    }

    #[test]
    fn test_column_list() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "TODO",
            ])
            .assert()
            .success();

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "DONE",
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "list",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["count"], 2);
    }

    #[test]
    fn test_column_reorder() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let col1_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "First",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let col1_json = parse_json_output(&String::from_utf8_lossy(&col1_output));
        let col1_id = extract_id(&col1_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "Second",
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "reorder",
                "--id",
                &col1_id,
                "--position",
                "1",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["position"], 1);
    }

    #[test]
    fn test_column_delete() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "To Delete",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let column_id = extract_id(&create_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "delete",
                "--id",
                &column_id,
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"deleted\""));
    }
}

mod card_tests {
    use super::*;

    fn setup_board_and_column(file: &std::path::Path) -> (String, String) {
        let board_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let board_json = parse_json_output(&String::from_utf8_lossy(&board_output));
        let board_id = extract_id(&board_json);

        let column_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "TODO",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let column_json = parse_json_output(&String::from_utf8_lossy(&column_output));
        let column_id = extract_id(&column_json);

        (board_id, column_id)
    }

    #[test]
    fn test_card_create() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Test Task",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["title"], "Test Task");
        assert_eq!(json["data"]["column_id"], column_id);
    }

    #[test]
    fn test_card_create_with_options() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Test Task",
                "--description",
                "A test description",
                "--priority",
                "high",
                "--points",
                "5",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["description"], "A test description");
        assert_eq!(json["data"]["priority"], "High");
        assert_eq!(json["data"]["points"], 5);
    }

    #[test]
    fn test_card_list() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task 1",
            ])
            .assert()
            .success();

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task 2",
            ])
            .assert()
            .success();

        let output = kanban()
            .args(["--file", file.to_str().unwrap(), "card", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["count"], 2);
    }

    #[test]
    fn test_card_update() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Original",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let card_id = extract_id(&create_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "update",
                "--id",
                &card_id,
                "--title",
                "Updated",
                "--priority",
                "critical",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["title"], "Updated");
        assert_eq!(json["data"]["priority"], "Critical");
    }

    #[test]
    fn test_card_move() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let col2_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "DONE",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let col2_json = parse_json_output(&String::from_utf8_lossy(&col2_output));
        let col2_id = extract_id(&col2_json);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let card_id = extract_id(&create_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "move",
                "--id",
                &card_id,
                "--column-id",
                &col2_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["column_id"], col2_id);
    }

    #[test]
    fn test_card_archive_and_restore() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "To Archive",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let card_id = extract_id(&create_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "archive",
                "--id",
                &card_id,
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"archived\""));

        let archived_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "list",
                "--archived",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let archived_json = parse_json_output(&String::from_utf8_lossy(&archived_output));
        assert_eq!(archived_json["data"]["count"], 1);

        let restore_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "restore",
                "--id",
                &card_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let restore_json = parse_json_output(&String::from_utf8_lossy(&restore_output));
        assert!(restore_json["success"].as_bool().unwrap());
        assert_eq!(restore_json["data"]["title"], "To Archive");
    }

    #[test]
    fn test_card_branch_name() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "My Feature Task",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let card_id = extract_id(&create_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "branch-name",
                "--id",
                &card_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"]["branch_name"]
            .as_str()
            .unwrap()
            .contains("my-feature-task"));
    }

    #[test]
    fn test_card_git_checkout() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "My Task",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let card_id = extract_id(&create_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "git-checkout",
                "--id",
                &card_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"]["command"]
            .as_str()
            .unwrap()
            .starts_with("git checkout -b"));
    }

    #[test]
    fn test_card_bulk_archive() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let card1_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task 1",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let card1_json = parse_json_output(&String::from_utf8_lossy(&card1_output));
        let card1_id = extract_id(&card1_json);

        let card2_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task 2",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let card2_json = parse_json_output(&String::from_utf8_lossy(&card2_output));
        let card2_id = extract_id(&card2_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "bulk-archive",
                "--ids",
                &format!("{},{}", card1_id, card2_id),
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["archived_count"], 2);
    }

    #[test]
    fn test_card_bulk_move() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let col2_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "DONE",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let col2_json = parse_json_output(&String::from_utf8_lossy(&col2_output));
        let col2_id = extract_id(&col2_json);

        let card1_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task 1",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let card1_json = parse_json_output(&String::from_utf8_lossy(&card1_output));
        let card1_id = extract_id(&card1_json);

        let card2_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task 2",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let card2_json = parse_json_output(&String::from_utf8_lossy(&card2_output));
        let card2_id = extract_id(&card2_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "bulk-move",
                "--ids",
                &format!("{},{}", card1_id, card2_id),
                "--column-id",
                &col2_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["moved_count"], 2);
    }
}

mod sprint_tests {
    use super::*;

    fn setup_board(file: &std::path::Path) -> String {
        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        extract_id(&json)
    }

    #[test]
    fn test_sprint_create() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["board_id"], board_id);
        assert_eq!(json["data"]["status"], "Planning");
    }

    #[test]
    fn test_sprint_create_with_options() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
                "--prefix",
                "SPRINT",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["prefix"], "SPRINT");
    }

    #[test]
    fn test_sprint_list() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success();

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "list",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["count"], 2);
    }

    #[test]
    fn test_sprint_activate() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let sprint_id = extract_id(&create_json);

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "activate",
                "--id",
                &sprint_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["status"], "Active");
        assert!(json["data"]["start_date"].as_str().is_some());
        assert!(json["data"]["end_date"].as_str().is_some());
    }

    #[test]
    fn test_sprint_complete() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let sprint_id = extract_id(&create_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "activate",
                "--id",
                &sprint_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "complete",
                "--id",
                &sprint_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["status"], "Completed");
    }

    #[test]
    fn test_sprint_cancel() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let create_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let sprint_id = extract_id(&create_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "activate",
                "--id",
                &sprint_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "cancel",
                "--id",
                &sprint_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["status"], "Cancelled");
    }

    #[test]
    fn test_card_assign_sprint() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let column_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "TODO",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let column_json = parse_json_output(&String::from_utf8_lossy(&column_output));
        let column_id = extract_id(&column_json);

        let card_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let card_json = parse_json_output(&String::from_utf8_lossy(&card_output));
        let card_id = extract_id(&card_json);

        let sprint_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let sprint_json = parse_json_output(&String::from_utf8_lossy(&sprint_output));
        let sprint_id = extract_id(&sprint_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "activate",
                "--id",
                &sprint_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "assign-sprint",
                "--id",
                &card_id,
                "--sprint-id",
                &sprint_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["sprint_id"], sprint_id);
    }

    #[test]
    fn test_card_unassign_sprint() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let column_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "column",
                "create",
                "--board-id",
                &board_id,
                "--name",
                "TODO",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let column_json = parse_json_output(&String::from_utf8_lossy(&column_output));
        let column_id = extract_id(&column_json);

        let card_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "create",
                "--board-id",
                &board_id,
                "--column-id",
                &column_id,
                "--title",
                "Task",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let card_json = parse_json_output(&String::from_utf8_lossy(&card_output));
        let card_id = extract_id(&card_json);

        let sprint_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let sprint_json = parse_json_output(&String::from_utf8_lossy(&sprint_output));
        let sprint_id = extract_id(&sprint_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "sprint",
                "activate",
                "--id",
                &sprint_id,
            ])
            .assert()
            .success();

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "assign-sprint",
                "--id",
                &card_id,
                "--sprint-id",
                &sprint_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "card",
                "unassign-sprint",
                "--id",
                &card_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"]["sprint_id"].is_null());
    }
}

mod export_import_tests {
    use super::*;

    #[test]
    fn test_export_full() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success();

        kanban()
            .args(["--file", file.to_str().unwrap(), "export"])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"boards\""))
            .stdout(predicate::str::contains("\"columns\""))
            .stdout(predicate::str::contains("Test Board"));
    }

    #[test]
    fn test_export_single_board() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        let board_output = kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let board_json = parse_json_output(&String::from_utf8_lossy(&board_output));
        let board_id = extract_id(&board_json);

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "export",
                "--board-id",
                &board_id,
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"boards\""))
            .stdout(predicate::str::contains("Test Board"));
    }

    #[test]
    fn test_import() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let import_file = dir.path().join("import.json");

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Original Board",
            ])
            .assert()
            .success();

        let export_output = kanban()
            .args(["--file", file.to_str().unwrap(), "export"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        fs::write(&import_file, String::from_utf8_lossy(&export_output).as_ref()).unwrap();

        let new_file = dir.path().join("new.json");
        let output = kanban()
            .args([
                "--file",
                new_file.to_str().unwrap(),
                "import",
                "--file",
                import_file.to_str().unwrap(),
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
    }
}

mod error_tests {
    use super::*;

    #[test]
    fn test_missing_required_args() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban()
            .args(["--file", file.to_str().unwrap(), "board", "create"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("--name"));
    }

    #[test]
    fn test_invalid_uuid() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "get",
                "--id",
                "not-a-uuid",
            ])
            .assert()
            .failure();
    }

    #[test]
    fn test_nonexistent_board() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban()
            .args([
                "--file",
                file.to_str().unwrap(),
                "board",
                "get",
                "--id",
                "00000000-0000-0000-0000-000000000000",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("not found"));
    }

    #[test]
    fn test_column_list_requires_board_id() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban()
            .args(["--file", file.to_str().unwrap(), "column", "list"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("--board-id"));
    }
}
