use assert_cmd::{cargo_bin_cmd, Command};
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn kanban() -> Command {
    cargo_bin_cmd!("kanban")
}

fn parse_json_output(output: &str) -> Value {
    serde_json::from_str(output).expect("Failed to parse JSON output")
}

fn extract_id(json: &Value) -> String {
    json["data"]["id"].as_str().unwrap().to_string()
}

fn kanban_no_config(dir: &std::path::Path) -> Command {
    let mut cmd = kanban();
    cmd.current_dir(dir)
        .env_remove("KANBAN_FILE")
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", dir);
    cmd
}

mod board_tests {
    use super::*;

    #[test]
    fn test_board_create() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban().args([file.to_str().unwrap()]).assert().success();
        let output = kanban()
            .args([
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

        kanban().args([file.to_str().unwrap()]).assert().success();
        let create_output = kanban()
            .args([
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
                file.to_str().unwrap(),
                "board",
                "update",
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

        kanban().args([file.to_str().unwrap()]).assert().success();

        let output = kanban()
            .args([file.to_str().unwrap(), "board", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["total"], 0);
    }

    #[test]
    fn test_board_list_with_data() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban().args([file.to_str().unwrap()]).assert().success();
        kanban()
            .args([
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
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Board 2",
            ])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "board", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["total"], 2);
    }

    #[test]
    fn test_board_get() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban().args([file.to_str().unwrap()]).assert().success();
        let create_output = kanban()
            .args([
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
            .args([file.to_str().unwrap(), "board", "get", &board_id])
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

        kanban().args([file.to_str().unwrap()]).assert().success();
        let create_output = kanban()
            .args([
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
                file.to_str().unwrap(),
                "board",
                "update",
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

        kanban().args([file.to_str().unwrap()]).assert().success();
        let create_output = kanban()
            .args([
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
            .args([file.to_str().unwrap(), "board", "delete", &board_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"deleted\""));

        let list_output = kanban()
            .args([file.to_str().unwrap(), "board", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&list_output));
        assert_eq!(json["data"]["total"], 0);
    }
}

mod column_tests {
    use super::*;

    fn setup_board(file: &std::path::Path) -> String {
        kanban().args([file.to_str().unwrap()]).assert().success();
        let output = kanban()
            .args([
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
                &board_id,
                "--name",
                "TODO",
            ])
            .assert()
            .success();

        kanban()
            .args([
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
                &board_id,
                "--name",
                "DONE",
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                file.to_str().unwrap(),
                "column",
                "list",
                "--board",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["total"], 2);
    }

    #[test]
    fn test_column_reorder() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let col1_output = kanban()
            .args([
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
                &board_id,
                "--name",
                "Second",
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                file.to_str().unwrap(),
                "column",
                "reorder",
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
            .args([file.to_str().unwrap(), "column", "delete", &column_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"deleted\""));
    }
}

mod card_tests {
    use super::*;

    fn setup_board_and_column(file: &std::path::Path) -> (String, String) {
        kanban().args([file.to_str().unwrap()]).assert().success();
        let board_output = kanban()
            .args([
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Task 1",
            ])
            .assert()
            .success();

        kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Task 2",
            ])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "card", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["total"], 2);
        assert!(json["data"]["total_pages"].is_number());
    }

    #[test]
    fn test_card_list_summary_omits_description() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Task With Desc",
                "--description",
                "Hello description",
            ])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "card", "list"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(!json["data"]["items"][0]
            .as_object()
            .unwrap()
            .contains_key("description"));
    }

    #[test]
    fn test_card_list_pagination() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        for i in 1..=3 {
            kanban()
                .args([
                    file.to_str().unwrap(),
                    "card",
                    "create",
                    "--board",
                    &board_id,
                    "--column",
                    &column_id,
                    "--title",
                    &format!("Task {i}"),
                ])
                .assert()
                .success();
        }

        let page1_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "list",
                "--page-size",
                "2",
                "--page",
                "1",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let page1_json = parse_json_output(&String::from_utf8_lossy(&page1_output));
        assert_eq!(page1_json["data"]["total"], 3);
        assert_eq!(page1_json["data"]["total_pages"], 2);
        assert_eq!(page1_json["data"]["items"].as_array().unwrap().len(), 2);

        let page2_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "list",
                "--page-size",
                "2",
                "--page",
                "2",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let page2_json = parse_json_output(&String::from_utf8_lossy(&page2_output));
        assert_eq!(page2_json["data"]["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_card_list_out_of_bounds_page() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Only Card",
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "list",
                "--page-size",
                "5",
                "--page",
                "99",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert_eq!(json["data"]["items"].as_array().unwrap().len(), 0);
        assert_eq!(json["data"]["total"], 1);
    }

    #[test]
    fn test_card_update() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let create_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "update",
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "move",
                &card_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
            .args([file.to_str().unwrap(), "card", "archive", &card_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"archived\""));

        let archived_output = kanban()
            .args([file.to_str().unwrap(), "card", "list", "--archived"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let archived_json = parse_json_output(&String::from_utf8_lossy(&archived_output));
        assert_eq!(archived_json["data"]["total"], 1);
        assert!(archived_json["data"]["items"][0]["archived_at"].is_string());
        assert!(archived_json["data"]["items"][0]["original_column_id"].is_string());
        assert!(!archived_json["data"]["items"][0]["card"]
            .as_object()
            .unwrap()
            .contains_key("description"));

        let restore_output = kanban()
            .args([file.to_str().unwrap(), "card", "restore", &card_id])
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
            .args([file.to_str().unwrap(), "card", "branch-name", &card_id])
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
            .args([file.to_str().unwrap(), "card", "git-checkout", &card_id])
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
    fn test_card_archive_cards() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let card1_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "archive-cards",
                "--cards",
                &format!("{},{}", card1_id, card2_id),
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["succeeded_count"], 2);
        assert_eq!(json["data"]["failed_count"], 0);
    }

    #[test]
    fn test_card_move_cards() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let col2_output = kanban()
            .args([
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "card",
                "move-cards",
                "--cards",
                &format!("{},{}", card1_id, card2_id),
                "--column",
                &col2_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["succeeded_count"], 2);
        assert_eq!(json["data"]["failed_count"], 0);
    }

    fn setup_board_and_column_with_prefix(
        file: &std::path::Path,
        prefix: &str,
    ) -> (String, String) {
        kanban().args([file.to_str().unwrap()]).assert().success();
        let board_output = kanban()
            .args([
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
                "--card-prefix",
                prefix,
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
    fn test_card_get_by_prefix_identifier() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column_with_prefix(&file, "KAN");

        let create_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Prefix Test Card",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let card_uuid = extract_id(&create_json);

        let output = kanban()
            .args([file.to_str().unwrap(), "card", "get", "KAN-1"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["id"], card_uuid);
        assert_eq!(json["data"]["title"], "Prefix Test Card");
    }

    #[test]
    fn test_card_get_by_number_only() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column_with_prefix(&file, "KAN");

        let create_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Number Lookup Card",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let card_uuid = extract_id(&create_json);

        let output = kanban()
            .args([file.to_str().unwrap(), "card", "get", "1"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["id"], card_uuid);
        assert_eq!(json["data"]["title"], "Number Lookup Card");
    }

    #[test]
    fn test_card_archive_by_identifier() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column_with_prefix(&file, "KAN");

        let create_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Archive Me",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let create_json = parse_json_output(&String::from_utf8_lossy(&create_output));
        let card_uuid = extract_id(&create_json);

        let output = kanban()
            .args([file.to_str().unwrap(), "card", "archive", "KAN-1"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["archived"], card_uuid);
    }

    fn setup_two_boards_same_prefix(
        file: &std::path::Path,
    ) -> (String, String, String, String, String, String) {
        let (board_a_id, col_a_id) = setup_board_and_column_with_prefix(file, "KAN");
        let (board_b_id, col_b_id) = {
            let board_output = kanban()
                .args([
                    file.to_str().unwrap(),
                    "board",
                    "create",
                    "--name",
                    "Board B",
                    "--card-prefix",
                    "KAN",
                ])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone();
            let board_json = parse_json_output(&String::from_utf8_lossy(&board_output));
            let board_b_id = extract_id(&board_json);
            let col_output = kanban()
                .args([
                    file.to_str().unwrap(),
                    "column",
                    "create",
                    "--board",
                    &board_b_id,
                    "--name",
                    "TODO",
                ])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone();
            let col_json = parse_json_output(&String::from_utf8_lossy(&col_output));
            (board_b_id, extract_id(&col_json))
        };

        let card_a_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_a_id,
                "--column",
                &col_a_id,
                "--title",
                "Card on A",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let card_a_id = extract_id(&parse_json_output(&String::from_utf8_lossy(&card_a_output)));

        let card_b_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_b_id,
                "--column",
                &col_b_id,
                "--title",
                "Card on B",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let card_b_id = extract_id(&parse_json_output(&String::from_utf8_lossy(&card_b_output)));

        (
            board_a_id, col_a_id, board_b_id, col_b_id, card_a_id, card_b_id,
        )
    }

    #[test]
    fn test_card_get_ambiguous_identifier_returns_all_matches() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (_, _, _, _, _, _) = setup_two_boards_same_prefix(&file);

        let output = kanban()
            .args([file.to_str().unwrap(), "card", "get", "KAN-1"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        let items = json["data"]
            .as_array()
            .expect("data should be an array when multiple cards match");
        assert_eq!(items.len(), 2);
        let titles: Vec<&str> = items.iter().map(|c| c["title"].as_str().unwrap()).collect();
        assert!(titles.contains(&"Card on A"));
        assert!(titles.contains(&"Card on B"));
    }

    #[test]
    fn test_card_mutate_ambiguous_identifier_error_lists_candidates() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (_, _, _, _, card_a_id, card_b_id) = setup_two_boards_same_prefix(&file);

        kanban()
            .args([file.to_str().unwrap(), "card", "archive", "KAN-1"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("ambiguous"))
            .stderr(predicate::str::contains(&card_a_id))
            .stderr(predicate::str::contains(&card_b_id));
    }
}

mod sprint_tests {
    use super::*;

    fn setup_board(file: &std::path::Path) -> String {
        kanban().args([file.to_str().unwrap()]).assert().success();
        let output = kanban()
            .args([
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
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
                &board_id,
            ])
            .assert()
            .success();

        kanban()
            .args([
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
                &board_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([
                file.to_str().unwrap(),
                "sprint",
                "list",
                "--board",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["total"], 2);
    }

    #[test]
    fn test_sprint_activate() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let board_id = setup_board(&file);

        let create_output = kanban()
            .args([
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
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
            .args([file.to_str().unwrap(), "sprint", "activate", &sprint_id])
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
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
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
            .args([file.to_str().unwrap(), "sprint", "activate", &sprint_id])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "sprint", "complete", &sprint_id])
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
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
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
            .args([file.to_str().unwrap(), "sprint", "activate", &sprint_id])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "sprint", "cancel", &sprint_id])
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
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
            .args([file.to_str().unwrap(), "sprint", "activate", &sprint_id])
            .assert()
            .success();

        let output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "assign-sprint",
                &card_id,
                "--sprint",
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
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
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
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
            .args([file.to_str().unwrap(), "sprint", "activate", &sprint_id])
            .assert()
            .success();

        kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "assign-sprint",
                &card_id,
                "--sprint",
                &sprint_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "card", "unassign-sprint", &card_id])
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

        kanban().args([file.to_str().unwrap()]).assert().success();
        kanban()
            .args([
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Test Board",
            ])
            .assert()
            .success();

        kanban()
            .args([file.to_str().unwrap(), "export"])
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

        kanban().args([file.to_str().unwrap()]).assert().success();
        let board_output = kanban()
            .args([
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
            .args([file.to_str().unwrap(), "export", "--board", &board_id])
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

        kanban().args([file.to_str().unwrap()]).assert().success();
        kanban()
            .args([
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "Original Board",
            ])
            .assert()
            .success();

        let export_output = kanban()
            .args([file.to_str().unwrap(), "export"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        fs::write(
            &import_file,
            String::from_utf8_lossy(&export_output).as_ref(),
        )
        .unwrap();

        let new_file = dir.path().join("new.json");
        kanban()
            .args([new_file.to_str().unwrap()])
            .assert()
            .success();

        let output = kanban()
            .args([
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
            .args([file.to_str().unwrap(), "board", "create"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("--name"));
    }

    #[test]
    fn test_invalid_uuid() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban()
            .args([file.to_str().unwrap(), "board", "get", "not-a-uuid"])
            .assert()
            .failure();
    }

    #[test]
    fn test_nonexistent_board() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban().args([file.to_str().unwrap()]).assert().success();
        kanban()
            .args([
                file.to_str().unwrap(),
                "board",
                "get",
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
            .args([file.to_str().unwrap(), "column", "list"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("--board"));
    }

    #[test]
    fn test_card_get_nonexistent_numeric_identifier() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        kanban().args([file.to_str().unwrap()]).assert().success();

        kanban()
            .args([file.to_str().unwrap(), "card", "get", "555"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("Card not found"))
            .stderr(predicate::str::contains("555"));
    }

    #[test]
    fn test_card_get_nonexistent_prefix_identifier() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        kanban().args([file.to_str().unwrap()]).assert().success();

        kanban()
            .args([file.to_str().unwrap(), "card", "get", "KAN-5"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("Card not found"))
            .stderr(predicate::str::contains("KAN-5"));
    }

    #[test]
    fn test_card_get_no_backtrace() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");

        kanban()
            .env("RUST_BACKTRACE", "1")
            .args([file.to_str().unwrap(), "card", "get", "555"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("stack backtrace").not());
    }

    fn setup_board_and_column(file: &std::path::Path) -> (String, String) {
        kanban().args([file.to_str().unwrap()]).assert().success();
        let board_output = kanban()
            .args([
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
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
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
    fn test_card_create_invalid_priority() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Test",
                "--priority",
                "badpriority",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("Invalid priority"))
            .stderr(predicate::str::contains("badpriority"));
    }

    #[test]
    fn test_card_list_invalid_status() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        kanban().args([file.to_str().unwrap()]).assert().success();

        kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "list",
                "--status",
                "notastatus",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("Invalid status"))
            .stderr(predicate::str::contains("notastatus"));
    }

    #[test]
    fn test_card_update_invalid_priority() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (board_id, column_id) = setup_board_and_column(&file);

        let card_output = kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "create",
                "--board",
                &board_id,
                "--column",
                &column_id,
                "--title",
                "Test",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let card_json = parse_json_output(&String::from_utf8_lossy(&card_output));
        let card_id = extract_id(&card_json);

        kanban()
            .args([
                file.to_str().unwrap(),
                "card",
                "update",
                &card_id,
                "--priority",
                "badpriority",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("Invalid priority"))
            .stderr(predicate::str::contains("badpriority"));
    }

    fn setup_sprint(file: &std::path::Path) -> String {
        kanban().args([file.to_str().unwrap()]).assert().success();
        let board_output = kanban()
            .args([
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

        let sprint_output = kanban()
            .args([
                file.to_str().unwrap(),
                "sprint",
                "create",
                "--board",
                &board_id,
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let sprint_json = parse_json_output(&String::from_utf8_lossy(&sprint_output));
        extract_id(&sprint_json)
    }

    #[test]
    fn test_sprint_update_invalid_start_date() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let sprint_id = setup_sprint(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "sprint",
                "update",
                &sprint_id,
                "--start-date",
                "notadate",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("Invalid date"))
            .stderr(predicate::str::contains("notadate"));
    }

    #[test]
    fn test_sprint_update_invalid_end_date() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let sprint_id = setup_sprint(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "sprint",
                "update",
                &sprint_id,
                "--end-date",
                "notadate",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("Invalid date"))
            .stderr(predicate::str::contains("notadate"));
    }
}

mod no_file_tests {
    use super::*;

    // Runs `kanban <args>` in a fresh temp dir with no KANBAN_FILE env var and
    // HOME pointing at the same empty dir (so no config file is found).
    fn kanban_no_config(dir: &std::path::Path) -> Command {
        let mut cmd = kanban();
        cmd.current_dir(dir)
            .env_remove("KANBAN_FILE")
            .env_remove("XDG_CONFIG_HOME")
            .env("HOME", dir);
        cmd
    }

    #[test]
    fn test_subcommand_with_no_file_and_no_config_fails_with_actionable_message() {
        let dir = tempdir().unwrap();
        kanban_no_config(dir.path())
            .args(["board", "list"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No data file specified"))
            .stderr(predicate::str::contains("KANBAN_FILE"));
        assert!(
            !dir.path().join("kanban.json").exists(),
            "must not silently create kanban.json"
        );
    }

    #[test]
    fn test_kanban_file_env_var_accepted_without_positional_arg() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("via-env.json");
        // Seed the file via the positional path so this test verifies env-var
        // resolution rather than implicit JSON auto-create-on-open.
        kanban_no_config(dir.path())
            .args([file.to_str().unwrap()])
            .assert()
            .success();
        kanban_no_config(dir.path())
            .args([
                file.to_str().unwrap(),
                "board",
                "create",
                "--name",
                "SeededViaPositional",
            ])
            .assert()
            .success();
        kanban_no_config(dir.path())
            .env("KANBAN_FILE", file.to_str().unwrap())
            .args(["board", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains("SeededViaPositional"));
    }

    #[test]
    fn test_completions_subcommand_requires_no_file() {
        let dir = tempdir().unwrap();
        kanban_no_config(dir.path())
            .args(["completions", "bash"])
            .assert()
            .success();
    }
}

mod version_and_help_tests {
    use super::*;

    fn kanban_no_config(dir: &std::path::Path) -> Command {
        let mut cmd = kanban();
        cmd.current_dir(dir)
            .env_remove("KANBAN_FILE")
            .env_remove("XDG_CONFIG_HOME")
            .env("HOME", dir);
        cmd
    }

    // The version output must go to stdout with a clean exit, no
    // "Error:" prefix, and a single trailing newline. Both -V and
    // --version must behave identically.
    #[test]
    fn test_short_version_flag_writes_clean_to_stdout() {
        let dir = tempdir().unwrap();
        let assert = kanban_no_config(dir.path()).args(["-V"]).assert().success();
        let output = assert.get_output();
        assert!(
            output.stderr.is_empty(),
            "stderr must be empty for -V, got: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.starts_with("kanban "),
            "stdout must start with \"kanban \", got: {:?}",
            stdout
        );
        assert!(
            !stdout.starts_with("Error:"),
            "stdout must not start with \"Error:\", got: {:?}",
            stdout
        );
        assert!(
            stdout.ends_with('\n') && !stdout.ends_with("\n\n"),
            "stdout must end with exactly one newline, got: {:?}",
            stdout
        );
    }

    #[test]
    fn test_long_version_flag_writes_clean_to_stdout() {
        let dir = tempdir().unwrap();
        let assert = kanban_no_config(dir.path())
            .args(["--version"])
            .assert()
            .success();
        let output = assert.get_output();
        assert!(
            output.stderr.is_empty(),
            "stderr must be empty for --version, got: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.starts_with("kanban "),
            "stdout must start with \"kanban \", got: {:?}",
            stdout
        );
        assert!(
            !stdout.starts_with("Error:"),
            "stdout must not start with \"Error:\", got: {:?}",
            stdout
        );
        assert!(
            stdout.ends_with('\n') && !stdout.ends_with("\n\n"),
            "stdout must end with exactly one newline, got: {:?}",
            stdout
        );
    }

    // --help must also reach stdout with exit 0 — same clap pitfall.
    #[test]
    fn test_help_flag_writes_clean_to_stdout() {
        let dir = tempdir().unwrap();
        let assert = kanban_no_config(dir.path())
            .args(["--help"])
            .assert()
            .success();
        let output = assert.get_output();
        assert!(
            output.stderr.is_empty(),
            "stderr must be empty for --help, got: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.starts_with("Error:"),
            "--help stdout must not start with \"Error:\", got: {:?}",
            stdout
        );
        assert!(
            stdout.contains("Usage:"),
            "--help stdout must include a Usage: section, got: {:?}",
            stdout
        );
    }

    // Real argument errors continue to be treated as errors.
    #[test]
    fn test_unknown_flag_still_errors_to_stderr() {
        let dir = tempdir().unwrap();
        let assert = kanban_no_config(dir.path())
            .args(["--no-such-flag"])
            .assert()
            .failure();
        let output = assert.get_output();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.is_empty(),
            "an unknown flag must surface a stderr error message"
        );
    }
}

/// Tests that every entity-id argument across the CLI accepts a human-readable name
/// (board name, column name, sprint name, sprint number) — not just a UUID.
mod name_resolution_tests {
    use super::*;

    /// Initialize a fresh file, create one board and one TODO column on it. Returns
    /// (file path string, board_id, column_id).
    fn setup_named_board(name: &str, prefix: &str) -> (tempfile::TempDir, String, String, String) {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json").to_str().unwrap().to_string();
        kanban().args([&file]).assert().success();
        let bjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file,
                    "board",
                    "create",
                    "--name",
                    name,
                    "--card-prefix",
                    prefix,
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        let board_id = extract_id(&bjson);
        let cjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file, "column", "create", "--board", &board_id, "--name", "TODO",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        let column_id = extract_id(&cjson);
        (dir, file, board_id, column_id)
    }

    // ---------- Board ----------

    #[test]
    fn test_board_get_by_name() {
        let (_dir, file, board_id, _col) = setup_named_board("My Board", "MB");
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "board", "get", "My Board"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["id"], board_id);
    }

    #[test]
    fn test_board_get_by_name_case_insensitive() {
        let (_dir, file, board_id, _col) = setup_named_board("MyBoard", "MB");
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "board", "get", "myboard"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["id"], board_id);
    }

    #[test]
    fn test_board_get_unknown_lists_available() {
        let (_dir, file, _b, _c) = setup_named_board("Kanban", "KAN");
        let assert = kanban()
            .args([&file, "board", "get", "Personal"])
            .assert()
            .failure();
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        assert!(stderr.contains("not found"), "stderr: {stderr}");
        assert!(stderr.contains("Kanban"), "stderr: {stderr}");
    }

    #[test]
    fn test_board_update_by_name() {
        let (_dir, file, board_id, _col) = setup_named_board("Original", "ORG");
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "board", "update", "Original", "--card-prefix", "NEW"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["id"], board_id);
        assert_eq!(json["data"]["card_prefix"], "NEW");
    }

    #[test]
    fn test_board_delete_by_name() {
        let (_dir, file, _board, _col) = setup_named_board("DeleteMe", "DEL");
        kanban()
            .args([&file, "board", "delete", "DeleteMe"])
            .assert()
            .success();
        let assert = kanban()
            .args([&file, "board", "get", "DeleteMe"])
            .assert()
            .failure();
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        assert!(stderr.contains("not found"), "stderr: {stderr}");
    }

    // ---------- Column ----------

    #[test]
    fn test_column_create_with_board_name() {
        let (_dir, file, _board, _col) = setup_named_board("B", "B");
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "column", "create", "--board", "B", "--name", "Doing"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["name"], "Doing");
    }

    #[test]
    fn test_column_list_with_board_name() {
        let (_dir, file, _board, _col) = setup_named_board("MyB", "MB");
        kanban()
            .args([
                &file, "column", "create", "--board", "MyB", "--name", "Doing",
            ])
            .assert()
            .success();
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "column", "list", "--board", "MyB"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_column_get_by_name() {
        let (_dir, file, _board, column_id) = setup_named_board("B", "B");
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "column", "get", "TODO"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["id"], column_id);
    }

    #[test]
    fn test_column_get_ambiguous_across_boards_lists_boards() {
        let (_dir, file, _b, _c) = setup_named_board("Alpha", "A");
        kanban()
            .args([
                &file,
                "board",
                "create",
                "--name",
                "Beta",
                "--card-prefix",
                "B",
            ])
            .assert()
            .success();
        kanban()
            .args([
                &file, "column", "create", "--board", "Beta", "--name", "TODO",
            ])
            .assert()
            .success();
        let assert = kanban()
            .args([&file, "column", "get", "TODO"])
            .assert()
            .failure();
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        assert!(stderr.contains("ambiguous"), "stderr: {stderr}");
        assert!(stderr.contains("'Alpha'"), "stderr: {stderr}");
        assert!(stderr.contains("'Beta'"), "stderr: {stderr}");
    }

    #[test]
    fn test_column_get_unknown_lists_available() {
        let (_dir, file, _b, _c) = setup_named_board("B", "B");
        let assert = kanban()
            .args([&file, "column", "get", "Nope"])
            .assert()
            .failure();
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        assert!(stderr.contains("not found"), "stderr: {stderr}");
        assert!(stderr.contains("TODO"), "stderr: {stderr}");
    }

    // ---------- Card ----------

    #[test]
    fn test_card_create_with_board_and_column_names() {
        let (_dir, file, _b, _c) = setup_named_board("B", "B");
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file, "card", "create", "--board", "B", "--column", "TODO", "--title", "Hello",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["title"], "Hello");
    }

    #[test]
    fn test_card_list_filters_by_names() {
        let (_dir, file, _b, _c) = setup_named_board("B", "B");
        kanban()
            .args([
                &file, "card", "create", "--board", "B", "--column", "TODO", "--title", "T1",
            ])
            .assert()
            .success();
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "card", "list", "--board", "B", "--column", "TODO"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_card_move_with_column_name() {
        let (_dir, file, _b, _c) = setup_named_board("B", "KAN");
        kanban()
            .args([&file, "column", "create", "--board", "B", "--name", "Doing"])
            .assert()
            .success();
        let cjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file, "card", "create", "--board", "B", "--column", "TODO", "--title", "T",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        let card_id = extract_id(&cjson);
        let mjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "card", "move", &card_id, "--column", "Doing"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(mjson["data"]["id"], card_id);
    }

    #[test]
    fn test_sprint_get_ambiguous_name_across_boards_lists_both() {
        // KAN-400 review fix: cross-board sprint name ambiguity must name the
        // conflicting boards (was previously underspecified).
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json").to_str().unwrap().to_string();
        kanban().args([&file]).assert().success();
        kanban()
            .args([
                &file,
                "board",
                "create",
                "--name",
                "Alpha",
                "--card-prefix",
                "A",
            ])
            .assert()
            .success();
        kanban()
            .args([
                &file,
                "board",
                "create",
                "--name",
                "Beta",
                "--card-prefix",
                "B",
            ])
            .assert()
            .success();
        kanban()
            .args([
                &file,
                "sprint",
                "create",
                "--board",
                "Alpha",
                "--name",
                "shared-name",
            ])
            .assert()
            .success();
        kanban()
            .args([
                &file,
                "sprint",
                "create",
                "--board",
                "Beta",
                "--name",
                "shared-name",
            ])
            .assert()
            .success();
        let assert = kanban()
            .args([&file, "sprint", "get", "shared-name"])
            .assert()
            .failure();
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        assert!(stderr.contains("ambiguous"), "stderr: {stderr}");
        assert!(stderr.contains("'Alpha'"), "stderr: {stderr}");
        assert!(stderr.contains("'Beta'"), "stderr: {stderr}");
    }

    /// Regression: `card restore <archived_uuid> --column <name>` must work.
    /// The board-derivation helper used to chain via active cards only, which
    /// failed for archived cards. It now falls back to archived_card.original_column_id.
    #[test]
    fn test_card_restore_archived_with_column_name() {
        let (_dir, file, _b, _c) = setup_named_board("B", "KAN");
        kanban()
            .args([&file, "column", "create", "--board", "B", "--name", "Doing"])
            .assert()
            .success();
        let cjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file, "card", "create", "--board", "B", "--column", "TODO", "--title", "X",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        let card_uuid = extract_id(&cjson);
        kanban()
            .args([&file, "card", "archive", "KAN-1"])
            .assert()
            .success();
        // Archived cards aren't reachable via KAN-N identifier, so use UUID.
        // The --column name resolution must still succeed by chaining via the
        // archived card's original_column_id to derive the board.
        let rjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "card", "restore", &card_uuid, "--column", "Doing"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(rjson["data"]["id"], card_uuid);
    }

    #[test]
    fn test_card_assign_sprint_by_name() {
        let (_dir, file, _b, _c) = setup_named_board("B", "KAN");
        kanban()
            .args([&file, "sprint", "create", "--board", "B", "--name", "alpha"])
            .assert()
            .success();
        let cjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file, "card", "create", "--board", "B", "--column", "TODO", "--title", "T",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        let card_id = extract_id(&cjson);
        let ajson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file,
                    "card",
                    "assign-sprint",
                    &card_id,
                    "--sprint",
                    "alpha",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert!(ajson["data"]["sprint_id"].is_string());
    }

    #[test]
    fn test_card_assign_sprint_by_number() {
        let (_dir, file, _b, _c) = setup_named_board("B", "KAN");
        kanban()
            .args([&file, "sprint", "create", "--board", "B", "--name", "alpha"])
            .assert()
            .success();
        let cjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file, "card", "create", "--board", "B", "--column", "TODO", "--title", "T",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        let card_id = extract_id(&cjson);
        let ajson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "card", "assign-sprint", &card_id, "--sprint", "1"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert!(ajson["data"]["sprint_id"].is_string());
    }

    #[test]
    fn test_card_move_cards_with_card_identifiers_and_column_name() {
        let (_dir, file, _b, _c) = setup_named_board("B", "KAN");
        kanban()
            .args([&file, "column", "create", "--board", "B", "--name", "Doing"])
            .assert()
            .success();
        for title in ["T1", "T2"] {
            kanban()
                .args([
                    &file, "card", "create", "--board", "B", "--column", "TODO", "--title", title,
                ])
                .assert()
                .success();
        }
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file,
                    "card",
                    "move-cards",
                    "--cards",
                    "KAN-1,KAN-2",
                    "--column",
                    "Doing",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["succeeded_count"], 2);
    }

    #[test]
    fn test_card_move_cards_spanning_boards_errors_clearly() {
        let (_dir, file, _b, _c) = setup_named_board("Alpha", "A");
        kanban()
            .args([
                &file,
                "board",
                "create",
                "--name",
                "Beta",
                "--card-prefix",
                "B",
            ])
            .assert()
            .success();
        kanban()
            .args([
                &file, "column", "create", "--board", "Beta", "--name", "TODO",
            ])
            .assert()
            .success();
        kanban()
            .args([
                &file, "card", "create", "--board", "Alpha", "--column", "TODO", "--title", "A1",
            ])
            .assert()
            .success();
        kanban()
            .args([
                &file, "card", "create", "--board", "Beta", "--column", "TODO", "--title", "B1",
            ])
            .assert()
            .success();
        let assert = kanban()
            .args([
                &file,
                "card",
                "move-cards",
                "--cards",
                "A-1,B-1",
                "--column",
                "TODO",
            ])
            .assert()
            .failure();
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        assert!(stderr.contains("same board"), "stderr: {stderr}");
        assert!(stderr.contains("'Alpha'"), "stderr: {stderr}");
        assert!(stderr.contains("'Beta'"), "stderr: {stderr}");
    }

    // ---------- Sprint ----------

    #[test]
    fn test_sprint_create_with_board_name() {
        let (_dir, file, _b, _c) = setup_named_board("B", "B");
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "sprint", "create", "--board", "B", "--name", "alpha"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["sprint_number"], 1);
    }

    #[test]
    fn test_sprint_get_by_name() {
        let (_dir, file, _b, _c) = setup_named_board("B", "B");
        let sjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file, "sprint", "create", "--board", "B", "--name", "yarara",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        let sprint_id = extract_id(&sjson);
        let g = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "sprint", "get", "yarara"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(g["data"]["id"], sprint_id);
    }

    #[test]
    fn test_sprint_get_by_number() {
        let (_dir, file, _b, _c) = setup_named_board("B", "B");
        let sjson = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "sprint", "create", "--board", "B", "--name", "alpha"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        let sprint_id = extract_id(&sjson);
        let g = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "sprint", "get", "1"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(g["data"]["id"], sprint_id);
    }

    #[test]
    fn test_sprint_activate_by_name() {
        let (_dir, file, _b, _c) = setup_named_board("B", "B");
        kanban()
            .args([&file, "sprint", "create", "--board", "B", "--name", "alpha"])
            .assert()
            .success();
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([&file, "sprint", "activate", "alpha"])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert_eq!(json["data"]["status"], "Active");
    }

    #[test]
    fn test_sprint_carry_over_by_names() {
        let (_dir, file, _b, _c) = setup_named_board("B", "KAN");
        // sprint #1
        kanban()
            .args([&file, "sprint", "create", "--board", "B", "--name", "alpha"])
            .assert()
            .success();
        // sprint #2
        kanban()
            .args([&file, "sprint", "create", "--board", "B", "--name", "beta"])
            .assert()
            .success();
        // activate sprint #1, then complete it
        kanban()
            .args([&file, "sprint", "activate", "alpha"])
            .assert()
            .success();
        kanban()
            .args([&file, "sprint", "complete", "alpha"])
            .assert()
            .success();
        // carry over by names
        let json = parse_json_output(&String::from_utf8_lossy(
            &kanban()
                .args([
                    &file,
                    "sprint",
                    "carry-over",
                    "--from",
                    "alpha",
                    "--to",
                    "beta",
                ])
                .assert()
                .success()
                .get_output()
                .stdout,
        ));
        assert!(json["data"]["carried_over"].is_number());
    }

    // ---------- Export ----------

    #[test]
    fn test_export_with_board_name() {
        let (_dir, file, _b, _c) = setup_named_board("ExpBoard", "E");
        // export prints raw JSON (not the CliResponse envelope)
        let out = kanban()
            .args([&file, "export", "--board", "ExpBoard"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let parsed: Value = serde_json::from_slice(&out).expect("export output should be JSON");
        // Either {"boards": [...]} or a single board shape — just confirm parseable + non-empty.
        assert!(parsed.is_object() || parsed.is_array());
    }
}

mod missing_file_tests {
    use super::*;

    #[test]
    fn test_missing_file_gives_clear_error() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("doesntexist.json");
        kanban()
            .args([file.to_str().unwrap(), "card", "get", "KAN-1"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("\"success\":false"))
            .stderr(predicate::str::contains("Board file not found"));
    }

    #[test]
    fn test_board_create_requires_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("doesntexist.json");
        kanban()
            .args([file.to_str().unwrap(), "board", "create", "--name", "Test"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Board file not found"));
    }

    #[test]
    fn test_no_subcommand_creates_file_when_missing() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("new.json");
        assert!(!file.exists());
        kanban().args([file.to_str().unwrap()]).assert().success();
        assert!(
            file.exists(),
            "kanban <file> must create the file when missing"
        );
    }
}

mod init_tests {
    use super::*;

    #[test]
    fn test_init_creates_file_with_default_board() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("boards.json");

        let output = kanban()
            .args([file.to_str().unwrap(), "init"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        assert!(file.exists());
        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["data"]["name"], "My Board");
    }

    #[test]
    fn test_init_creates_file_with_named_board() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("boards.json");

        let output = kanban()
            .args([file.to_str().unwrap(), "init", "--board", "Sprint 1"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        assert!(file.exists());
        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert_eq!(json["data"]["name"], "Sprint 1");
    }

    #[test]
    fn test_init_via_kanban_file_env() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("env-board.json");

        kanban_no_config(dir.path())
            .env("KANBAN_FILE", file.to_str().unwrap())
            .args(["init", "--board", "Env Board"])
            .assert()
            .success();

        assert!(file.exists());
    }

    #[test]
    fn test_init_fails_cleanly_on_bad_path() {
        let dir = tempdir().unwrap();
        let bad = dir
            .path()
            .join("no")
            .join("such")
            .join("dir")
            .join("x.json");

        kanban()
            .args([bad.to_str().unwrap(), "init"])
            .assert()
            .failure();
    }
}

mod relation_tests {
    use super::*;

    fn setup_two_cards(file: &std::path::Path) -> (String, String) {
        kanban().args([file.to_str().unwrap()]).assert().success();

        let board_output = kanban()
            .args([
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
        let board_id = extract_id(&parse_json_output(&String::from_utf8_lossy(&board_output)));

        let column_output = kanban()
            .args([
                file.to_str().unwrap(),
                "column",
                "create",
                "--board",
                &board_id,
                "--name",
                "TODO",
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let column_id = extract_id(&parse_json_output(&String::from_utf8_lossy(&column_output)));

        let make_card = |title: &str| -> String {
            let out = kanban()
                .args([
                    file.to_str().unwrap(),
                    "card",
                    "create",
                    "--board",
                    &board_id,
                    "--column",
                    &column_id,
                    "--title",
                    title,
                ])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone();
            extract_id(&parse_json_output(&String::from_utf8_lossy(&out)))
        };
        let parent_id = make_card("Parent");
        let child_id = make_card("Child");
        (parent_id, child_id)
    }

    #[test]
    fn test_relation_add_creates_edge_visible_via_parents() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (parent_id, child_id) = setup_two_cards(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "relation",
                "add",
                "--parent",
                &parent_id,
                "--child",
                &child_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "relation", "parents", &child_id])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert!(json["success"].as_bool().unwrap());
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["id"], parent_id);
    }

    #[test]
    fn test_relation_add_cycle_returns_error_exit_code() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (a, b) = setup_two_cards(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "relation",
                "add",
                "--parent",
                &a,
                "--child",
                &b,
            ])
            .assert()
            .success();

        // Closing the cycle b -> a should fail.
        kanban()
            .args([
                file.to_str().unwrap(),
                "relation",
                "add",
                "--parent",
                &b,
                "--child",
                &a,
            ])
            .assert()
            .failure();
    }

    #[test]
    fn test_relation_remove_removes_edge() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (parent_id, child_id) = setup_two_cards(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "relation",
                "add",
                "--parent",
                &parent_id,
                "--child",
                &child_id,
            ])
            .assert()
            .success();

        kanban()
            .args([
                file.to_str().unwrap(),
                "relation",
                "remove",
                "--parent",
                &parent_id,
                "--child",
                &child_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "relation", "parents", &child_id])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let json = parse_json_output(&String::from_utf8_lossy(&output));
        assert_eq!(json["data"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_relation_children_returns_summaries() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (parent_id, child_id) = setup_two_cards(&file);

        kanban()
            .args([
                file.to_str().unwrap(),
                "relation",
                "add",
                "--parent",
                &parent_id,
                "--child",
                &child_id,
            ])
            .assert()
            .success();

        let output = kanban()
            .args([file.to_str().unwrap(), "relation", "children", &parent_id])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let json = parse_json_output(&String::from_utf8_lossy(&output));
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["id"], child_id);
        assert_eq!(data[0]["title"], "Child");
    }

    #[test]
    fn test_relation_add_requires_both_flags() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.json");
        let (parent_id, _) = setup_two_cards(&file);

        // Missing --child
        kanban()
            .args([
                file.to_str().unwrap(),
                "relation",
                "add",
                "--parent",
                &parent_id,
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("--child"));
    }
}
