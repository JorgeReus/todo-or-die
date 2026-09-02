use todo_or_die_core::{SourceComment, SourceSpan};
use tree_sitter::{Node, Parser};
fn language(p: &std::path::Path) -> Option<tree_sitter::Language> {
    match p.extension()?.to_str()? {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "ts" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        _ => None,
    }
}
fn walk(n: Node, s: &[u8], o: &mut Vec<SourceComment>) {
    if n.kind() == "comment" || n.kind().ends_with("_comment") {
        let raw = String::from_utf8_lossy(&s[n.byte_range()]).into_owned();
        let content = raw
            .trim()
            .trim_start_matches("//")
            .trim_start_matches('#')
            .trim_start_matches("/*")
            .trim_end_matches("*/")
            .trim()
            .trim_start_matches('*')
            .trim()
            .to_owned();
        let a = n.start_position();
        let b = n.end_position();
        o.push(SourceComment {
            raw_text: raw,
            content,
            span: SourceSpan {
                start_byte: n.start_byte(),
                end_byte: n.end_byte(),
                start_line: a.row,
                start_column: a.column,
                end_line: b.row,
                end_column: b.column,
            },
        })
    }
    let mut c = n.walk();
    for x in n.children(&mut c) {
        walk(x, s, o)
    }
}
fn walk_legacy(n: tree_sitter_legacy::Node, s: &[u8], o: &mut Vec<SourceComment>) {
    if n.kind() == "comment" || n.kind().ends_with("_comment") {
        let raw = String::from_utf8_lossy(&s[n.byte_range()]).into_owned();
        let content = raw.trim().trim_start_matches("//").trim().to_owned();
        let a = n.start_position();
        let b = n.end_position();
        o.push(SourceComment {
            raw_text: raw,
            content,
            span: SourceSpan {
                start_byte: n.start_byte(),
                end_byte: n.end_byte(),
                start_line: a.row,
                start_column: a.column,
                end_line: b.row,
                end_column: b.column,
            },
        });
    }
    let mut c = n.walk();
    for x in n.children(&mut c) {
        walk_legacy(x, s, o);
    }
}
pub fn extract_comments(p: &std::path::Path, s: &str) -> Result<Vec<SourceComment>, String> {
    if matches!(p.extension().and_then(|e| e.to_str()), Some("kt" | "kts")) {
        let mut q = tree_sitter_legacy::Parser::new();
        q.set_language(tree_sitter_kotlin::language())
            .map_err(|e| e.to_string())?;
        let t = q.parse(s, None).ok_or("failed to parse source")?;
        let mut o = vec![];
        walk_legacy(t.root_node(), s.as_bytes(), &mut o);
        return Ok(o);
    }
    let Some(l) = language(p) else {
        return Ok(vec![]);
    };
    let mut q = Parser::new();
    q.set_language(&l).map_err(|e| e.to_string())?;
    let t = q.parse(s, None).ok_or("failed to parse source")?;
    let mut o = vec![];
    walk(t.root_node(), s.as_bytes(), &mut o);
    Ok(o)
}
