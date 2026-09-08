#[cfg(test)]
mod tests {
    #[test]
    fn trait_is_object_safe() {
        fn _accepts_dyn(_: &mut dyn super::UndoOperations) {}
    }

    #[test]
    fn test_undo_operations_declares_every_method() {
        let src = include_str!("undo_operations.rs");
        let decl = &src[..src.find("#[cfg(test)]").unwrap()];
        assert_eq!(
            decl.matches("\n    fn ").count(),
            4,
            "UndoOperations must declare exactly 4 methods"
        );
        assert!(
            !decl.contains("\n    }"),
            "UndoOperations methods must have no default bodies"
        );
    }
}
