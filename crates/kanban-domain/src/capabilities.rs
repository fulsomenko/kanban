#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_wiring_after_a_mid_file_cfg_test_module_is_still_seen() {
        let src = "fn a() {}\n\n#[cfg(test)]\nmod t {\n    #[test]\n    fn x() { create_board(); }\n}\n\nfn wired() { archive_board(); }\n";
        let violations = capability_violations(&["archive_board", "create_board"], &[src], &[]);
        assert!(violations.iter().any(|v| v.contains("create_board")));
        assert!(!violations.iter().any(|v| v.contains("archive_board")));
    }

    #[test]
    fn test_wiring_inside_trailing_cfg_test_module_is_ignored() {
        let src = "fn r() {}\n\n#[cfg(test)]\nmod t {\n    fn x() { delete_card(); }\n}\n";
        let violations = capability_violations(&["delete_card"], &[src], &[]);
        assert!(violations.iter().any(|v| v.contains("delete_card")));
    }

    #[test]
    fn test_unwired_manifest_entry_is_reported() {
        let violations = capability_violations(&["archive_board"], &["fn nothing() {}"], &[]);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("archive_board"));
    }

    #[test]
    fn test_wired_entry_with_bare_call_passes() {
        let violations = capability_violations(&["update_board"], &["c.update_board(id, u)"], &[]);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_wired_entry_with_impl_suffix_passes() {
        let violations = capability_violations(
            &["update_board"],
            &["mutate(|c| c.update_board_impl(id, u))"],
            &[],
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn test_wired_entry_behind_an_underscore_prefix_passes() {
        let violations = capability_violations(&["undo"], &["pub async fn tool_undo(&self)"], &[]);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_entry_name_does_not_match_pluralized_sibling() {
        let violations = capability_violations(&["update_card"], &["c.update_cards(v)"], &[]);
        assert!(violations.iter().any(|v| v.contains("update_card")));
    }

    #[test]
    fn test_entry_name_does_not_match_longer_identifier_prefix() {
        let violations = capability_violations(&["block"], &["c.unblock(a, b)"], &[]);
        assert!(violations.iter().any(|v| v.contains("block")));
    }

    #[test]
    fn test_declined_entry_with_reason_passes() {
        let violations = capability_violations(
            &["undo"],
            &["fn nothing() {}"],
            &[("undo", "sessions are shared across clients")],
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn test_declined_entry_with_empty_reason_is_reported() {
        let violations =
            capability_violations(&["undo"], &["fn nothing() {}"], &[("undo", "")]);
        assert!(violations.iter().any(|v| v.contains("undo")));
    }

    #[test]
    fn test_declined_but_wired_entry_is_reported() {
        let violations = capability_violations(
            &["undo"],
            &["c.undo()"],
            &[("undo", "sessions are shared")],
        );
        assert!(violations.iter().any(|v| v.contains("undo")));
    }
}
