use todo_or_die_core::{SourceComment, SourceSpan};

#[derive(Clone, Copy, PartialEq)]
enum CommentProfile {
    CLike,
    PythonLike,
    Html,
    Svelte,
}

#[derive(Clone, Copy)]
enum CommentKind {
    Line,
    Block,
}

fn syntax(path: &std::path::Path) -> Option<CommentProfile> {
    match path.extension()?.to_str()? {
        "py" => Some(CommentProfile::PythonLike),
        "html" | "htm" => Some(CommentProfile::Html),
        "svelte" => Some(CommentProfile::Svelte),
        "rs" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "kt" | "kts" | "zig" => {
            Some(CommentProfile::CLike)
        }
        _ => None,
    }
}

fn span(source: &str, start: usize, end: usize) -> SourceSpan {
    let before = &source[..start];
    let text = &source[start..end];
    let line = before.bytes().filter(|b| *b == b'\n').count();
    let column = before.rsplit('\n').next().unwrap_or("").chars().count();
    let end_line = line + text.bytes().filter(|b| *b == b'\n').count();
    let end_column = if text.contains('\n') {
        text.rsplit('\n').next().unwrap_or("").chars().count()
    } else {
        column + text.chars().count()
    };
    SourceSpan {
        start_byte: start,
        end_byte: end,
        start_line: line,
        start_column: column,
        end_line,
        end_column,
    }
}

fn normalize(raw: &str, profile: CommentProfile, kind: CommentKind) -> String {
    let prefix = match profile {
        CommentProfile::CLike => "//",
        CommentProfile::PythonLike => "#",
        CommentProfile::Html | CommentProfile::Svelte => "<!--",
    };
    let body = match kind {
        CommentKind::Line => raw.trim().trim_start_matches(prefix),
        CommentKind::Block => {
            let (open, close) = if matches!(profile, CommentProfile::Html | CommentProfile::Svelte)
            {
                ("<!--", "-->")
            } else {
                ("/*", "*/")
            };
            raw.trim().trim_start_matches(open).trim_end_matches(close)
        }
    };
    body.lines()
        .map(|line| {
            line.trim()
                .trim_start_matches(prefix)
                .trim_start_matches('*')
                .trim()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

pub fn extract_comments(
    path: &std::path::Path,
    source: &str,
) -> Result<Vec<SourceComment>, String> {
    let Some(syntax) = syntax(path) else {
        return Ok(vec![]);
    };
    let bytes = source.as_bytes();
    let mut comments = vec![];
    let mut i = 0;
    let mut line_start = true;
    while i < bytes.len() {
        if matches!(syntax, CommentProfile::CLike | CommentProfile::Svelte)
            && i + 1 < bytes.len()
            && bytes[i..i + 2] == *b"\\\\"
        {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if matches!(syntax, CommentProfile::CLike | CommentProfile::Svelte)
            && i + 1 < bytes.len()
            && bytes[i] == b'/'
            && bytes[i + 1] == b'/'
        {
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            let raw = source[start..i].to_owned();
            comments.push(SourceComment {
                content: normalize(&raw, syntax, CommentKind::Line),
                raw_text: raw,
                span: span(source, start, i),
            });
            line_start = false;
            continue;
        }
        if matches!(syntax, CommentProfile::CLike | CommentProfile::Svelte)
            && i + 1 < bytes.len()
            && bytes[i] == b'/'
            && bytes[i + 1] == b'*'
        {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            let raw = source[start..i].to_owned();
            comments.push(SourceComment {
                content: normalize(&raw, syntax, CommentKind::Block),
                raw_text: raw,
                span: span(source, start, i),
            });
            line_start = false;
            continue;
        }
        if matches!(syntax, CommentProfile::Html | CommentProfile::Svelte)
            && i + 3 < bytes.len()
            && &bytes[i..i + 4] == b"<!--"
        {
            let start = i;
            i += 4;
            while i + 2 < bytes.len() && &bytes[i..i + 3] != b"-->" {
                i += 1;
            }
            i = (i + 3).min(bytes.len());
            let raw = source[start..i].to_owned();
            comments.push(SourceComment {
                content: normalize(&raw, syntax, CommentKind::Block),
                raw_text: raw,
                span: span(source, start, i),
            });
            line_start = false;
            continue;
        }
        if syntax == CommentProfile::PythonLike && bytes[i] == b'#' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            let raw = source[start..i].to_owned();
            comments.push(SourceComment {
                content: normalize(&raw, syntax, CommentKind::Line),
                raw_text: raw,
                span: span(source, start, i),
            });
            line_start = false;
            continue;
        }
        if bytes[i] == b'"'
            || bytes[i] == b'\''
            || (bytes[i] == b'`' && matches!(syntax, CommentProfile::CLike))
        {
            let quote = bytes[i];
            let triple = i + 2 < bytes.len() && bytes[i..i + 3] == [quote, quote, quote];
            i += if triple { 3 } else { 1 };
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if triple && i + 2 < bytes.len() && bytes[i..i + 3] == [quote, quote, quote] {
                    i += 3;
                    break;
                }
                if !triple && bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            line_start = false;
            continue;
        }
        if bytes[i] == b'\n' {
            line_start = true;
        } else if !bytes[i].is_ascii_whitespace() {
            line_start = false;
        }
        i += 1;
    }
    let _ = line_start;
    Ok(comments)
}
