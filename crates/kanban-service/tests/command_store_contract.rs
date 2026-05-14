use kanban_domain::command_store::CommandStore;
use kanban_domain::commands::{BoardCommand, Command, CreateBoard};
use kanban_domain::{InMemoryStore, Snapshot};
use uuid::Uuid;

fn make_cmd(name: &str) -> Command {
    Command::Board(BoardCommand::Create(CreateBoard {
        id: Uuid::new_v4(),
        name: name.into(),
        card_prefix: None,
        position: 0,
    }))
}

macro_rules! contract_tests {
    ($make_store:expr) => {
        #[test]
        fn test_append_and_load_commands() {
            let store = $make_store;
            store.append_commands(&[make_cmd("B1")]).unwrap();
            store.append_commands(&[make_cmd("B2")]).unwrap();

            let batches = store.load_commands(0, 2).unwrap();
            assert_eq!(batches.len(), 2);
            assert_eq!(batches[0].len(), 1);
            assert_eq!(batches[1].len(), 1);
        }

        #[test]
        fn test_load_commands_half_open_range() {
            let store = $make_store;
            store.append_commands(&[make_cmd("B1")]).unwrap();
            store.append_commands(&[make_cmd("B2")]).unwrap();
            store.append_commands(&[make_cmd("B3")]).unwrap();

            let batches = store.load_commands(1, 3).unwrap();
            assert_eq!(batches.len(), 2);

            let batches = store.load_commands(0, 1).unwrap();
            assert_eq!(batches.len(), 1);
        }

        #[test]
        fn test_truncate_commands_after() {
            let store = $make_store;
            store.append_commands(&[make_cmd("B1")]).unwrap();
            store.append_commands(&[make_cmd("B2")]).unwrap();
            store.append_commands(&[make_cmd("B3")]).unwrap();

            store.truncate_commands_after(2).unwrap();
            assert_eq!(store.command_count().unwrap(), 2);

            let batches = store.load_commands(0, 2).unwrap();
            assert_eq!(batches.len(), 2);
        }

        #[test]
        fn test_truncate_commands_after_zero_clears_all() {
            let store = $make_store;
            store.append_commands(&[make_cmd("B1")]).unwrap();
            store.append_commands(&[make_cmd("B2")]).unwrap();

            store.truncate_commands_after(0).unwrap();
            assert_eq!(store.command_count().unwrap(), 0);
        }

        #[test]
        fn test_shift_commands_renumbers() {
            let store = $make_store;
            store.append_commands(&[make_cmd("B1")]).unwrap();
            store.append_commands(&[make_cmd("B2")]).unwrap();
            store.append_commands(&[make_cmd("B3")]).unwrap();

            store.shift_commands(1).unwrap();
            assert_eq!(
                store.command_count().unwrap(),
                2,
                "shift_commands(1) should drop the first batch"
            );

            let batches = store.load_commands(0, 2).unwrap();
            assert_eq!(batches.len(), 2);
        }
    };
}

mod in_memory {
    use super::*;

    fn make_store() -> InMemoryStore {
        InMemoryStore::new()
    }

    contract_tests!(make_store());

    #[test]
    fn test_store_and_load_snapshot_at() {
        let store = make_store();
        assert!(
            store.supports_indexed_snapshots(),
            "InMemoryStore should support indexed snapshots"
        );

        let snap = Snapshot::new();
        store.store_snapshot_at(1, &snap).unwrap();
        let loaded = store.load_snapshot_at(1).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), snap);

        let missing = store.load_snapshot_at(99).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_shift_commands_removes_old_snapshots() {
        let store = make_store();
        store.append_commands(&[make_cmd("B1")]).unwrap();
        store.append_commands(&[make_cmd("B2")]).unwrap();
        store.append_commands(&[make_cmd("B3")]).unwrap();

        let snap = Snapshot::new();
        store.store_snapshot_at(1, &snap).unwrap();
        store.store_snapshot_at(2, &snap).unwrap();
        store.store_snapshot_at(3, &snap).unwrap();

        store.shift_commands(2).unwrap();

        assert!(
            store.load_snapshot_at(0).unwrap().is_none(),
            "snapshots for dropped indices should be removed"
        );
        assert!(
            store.load_snapshot_at(1).unwrap().is_some(),
            "snapshot at old index 3 should be renumbered to 1"
        );
    }
}
