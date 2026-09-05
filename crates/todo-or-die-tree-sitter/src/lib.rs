use todo_or_die_core::{SourceComment, SourceSpan};
use tree_sitter::{Node, Parser};
#[derive(Clone, Copy)]
enum CommentSyntax {
    Slash,
    Hash,
}

fn language(p: &std::path::Path) -> Option<(tree_sitter::Language, CommentSyntax)> {
    match p.extension()?.to_str()? {
        "rs" => Some((tree_sitter_rust::LANGUAGE.into(), CommentSyntax::Slash)),
        "ts" | "tsx" => Some((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            CommentSyntax::Slash,
        )),
        "js" | "jsx" => Some((
            tree_sitter_javascript::LANGUAGE.into(),
            CommentSyntax::Slash,
        )),
        "py" => Some((tree_sitter_python::LANGUAGE.into(), CommentSyntax::Hash)),
        "go" => Some((tree_sitter_go::LANGUAGE.into(), CommentSyntax::Slash)),
        "java" => Some((tree_sitter_java::LANGUAGE.into(), CommentSyntax::Slash)),
        "zig" => Some((tree_sitter_zig::LANGUAGE.into(), CommentSyntax::Slash)),
        _ => None,
    }
}
fn normalize(raw: &str, syntax: CommentSyntax) -> String {
    let line_prefix = match syntax {
        CommentSyntax::Slash => "//",
        CommentSyntax::Hash => "#",
    };
    raw.trim()
        .trim_start_matches("/*")
        .trim_end_matches("*/")
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches(line_prefix)
                .trim_start_matches('*')
                .trim()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

fn make_comment(
    raw: String,
    start_byte: usize,
    end_byte: usize,
    start: (usize, usize),
    end: (usize, usize),
    syntax: CommentSyntax,
) -> SourceComment {
    SourceComment {
        raw_text: raw.clone(),
        content: normalize(&raw, syntax),
        span: SourceSpan {
            start_byte,
            end_byte,
            start_line: start.0,
            start_column: start.1,
            end_line: end.0,
            end_column: end.1,
        },
    }
}

fn walk(n: Node, s: &[u8], syntax: CommentSyntax, o: &mut Vec<SourceComment>) {
    if n.kind() == "comment" || n.kind().ends_with("_comment") {
        let raw = String::from_utf8_lossy(&s[n.byte_range()]).into_owned();
        let a = n.start_position();
        let b = n.end_position();
        o.push(make_comment(
            raw,
            n.start_byte(),
            n.end_byte(),
            (a.row, a.column),
            (b.row, b.column),
            syntax,
        ));
    }
    let mut c = n.walk();
    for x in n.children(&mut c) {
        walk(x, s, syntax, o)
    }
}
fn walk_legacy(n: tree_sitter_legacy::Node, s: &[u8], o: &mut Vec<SourceComment>) {
    if n.kind() == "comment" || n.kind().ends_with("_comment") {
        let raw = String::from_utf8_lossy(&s[n.byte_range()]).into_owned();
        let a = n.start_position();
        let b = n.end_position();
        o.push(make_comment(
            raw,
            n.start_byte(),
            n.end_byte(),
            (a.row, a.column),
            (b.row, b.column),
            CommentSyntax::Slash,
        ));
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
    let Some((l, syntax)) = language(p) else {
        return Ok(vec![]);
    };
    let mut q = Parser::new();
    q.set_language(&l).map_err(|e| e.to_string())?;
    let t = q.parse(s, None).ok_or("failed to parse source")?;
    let mut o = vec![];
    walk(t.root_node(), s.as_bytes(), syntax, &mut o);
    Ok(o)
}
