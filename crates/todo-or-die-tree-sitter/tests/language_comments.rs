use std::{fs, path::Path};

#[test]
fn extracts_directives_from_real_comments_in_each_supported_language() {
    for extension in ["rs", "ts", "js", "py", "go", "java", "kt", "zig"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(format!("comment.{extension}"));
        let source = fs::read_to_string(&path).unwrap();
        let comments = todo_or_die_tree_sitter::extract_comments(&path, &source).unwrap();
        assert_eq!(
            comments
                .iter()
                .filter(|c| c.content.contains("TODO-OR-DIE: after 2099-01-01"))
                .count(),
            1,
            "{extension} {comments:?}"
        );
        assert_eq!(
            comments
                .iter()
                .filter(|c| c.content.contains("TODO-OR-DIE: after 2098-01-01"))
                .count(),
            1,
            "multiline comment: {extension} {comments:?}"
        );
        assert!(
            !comments
                .iter()
                .any(|c| c.content.contains("inside a string")),
            "{extension}"
        );
    }
}

#[test]
fn extracts_html_and_svelte_comments_without_strings() {
    for extension in ["html", "svelte"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(format!("comment.{extension}"));
        let source = fs::read_to_string(&path).unwrap();
        let comments = todo_or_die_tree_sitter::extract_comments(&path, &source).unwrap();
        assert_eq!(
            comments
                .iter()
                .filter(|c| c.content.contains("after 2099-01-01"))
                .count(),
            1,
            "{extension} {comments:?}"
        );
        assert!(!comments
            .iter()
            .any(|c| c.content.contains("inside a string")));
    }
}
