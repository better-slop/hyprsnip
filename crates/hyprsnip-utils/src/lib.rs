use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aggressiveness {
    Low,
    Normal,
    High,
}

impl Default for Aggressiveness {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrimOptions {
    pub aggressiveness: Aggressiveness,
    pub keep_blank_lines: bool,
    pub remove_box_drawing: bool,
    pub max_auto_lines: usize,
}

impl Default for TrimOptions {
    fn default() -> Self {
        Self {
            aggressiveness: Aggressiveness::Normal,
            keep_blank_lines: false,
            remove_box_drawing: true,
            max_auto_lines: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrimResult {
    pub original: String,
    pub trimmed: String,
    pub changed: bool,
    pub skipped: bool,
    pub reason: Option<String>,
}

impl Default for TrimResult {
    fn default() -> Self {
        Self {
            original: String::new(),
            trimmed: String::new(),
            changed: false,
            skipped: false,
            reason: None,
        }
    }
}

pub fn trim_text(input: &str, options: &TrimOptions) -> TrimResult {
    let original = input.to_string();
    let line_count = input.lines().count();

    if options.max_auto_lines > 0 && line_count > options.max_auto_lines {
        return TrimResult {
            original: original.clone(),
            trimmed: original,
            changed: false,
            skipped: true,
            reason: Some(format!(
                "skipped: line_count={} > max_auto_lines={}",
                line_count, options.max_auto_lines
            )),
        };
    }

    let mut lines: Vec<String> = input.lines().map(|line| line.to_string()).collect();

    if options.remove_box_drawing {
        for line in &mut lines {
            *line = strip_box_drawing(line);
        }
    }

    strip_prompt_prefix_in_place(&mut lines, options.aggressiveness);

    let trimmed = if options.keep_blank_lines {
        flatten_preserving_blank_lines(&lines)
    } else {
        flatten_dropping_blank_lines(&lines)
    };

    let changed = trimmed != original;

    TrimResult {
        original,
        trimmed,
        changed,
        skipped: false,
        reason: None,
    }
}

fn strip_box_drawing(line: &str) -> String {
    let mut s = line.trim().to_string();

    while let Some(first) = s.chars().next() {
        if first == '│' || first == '┃' {
            s = s[first.len_utf8()..].trim_start().to_string();
        } else {
            break;
        }
    }

    while let Some(last) = s.chars().last() {
        if last == '│' || last == '┃' {
            let last_len = last.len_utf8();
            s = s[..s.len().saturating_sub(last_len)].trim_end().to_string();
        } else {
            break;
        }
    }

    s
}

fn strip_prompt_prefix_in_place(lines: &mut [String], aggressiveness: Aggressiveness) {
    let Some((idx, line)) = lines
        .iter_mut()
        .enumerate()
        .find(|(_, line)| !line.trim().is_empty())
    else {
        return;
    };

    let trimmed = line.trim_start();
    let Some((prefix, rest)) = trimmed
        .strip_prefix("$ ")
        .map(|rest| ("$ ", rest))
        .or_else(|| trimmed.strip_prefix("% ").map(|rest| ("% ", rest)))
        .or_else(|| trimmed.strip_prefix("> ").map(|rest| ("> ", rest)))
        .or_else(|| trimmed.strip_prefix("# ").map(|rest| ("# ", rest)))
    else {
        return;
    };

    if prefix == "# " && looks_like_markdown_heading(trimmed) {
        return;
    }

    if !looks_command_like(rest, aggressiveness) {
        return;
    }

    // Preserve original indentation before the prompt.
    let indent_len = line.len().saturating_sub(trimmed.len());
    let indent = &line[..indent_len];
    *line = format!("{}{}", indent, rest);

    // If we stripped the prompt on the first non-empty line, also strip common prompt on
    // subsequent lines for multi-line copies.
    for later in lines.iter_mut().skip(idx + 1) {
        let later_trimmed = later.trim_start();
        let Some(rest) = later_trimmed
            .strip_prefix("$ ")
            .or_else(|| later_trimmed.strip_prefix("% "))
            .or_else(|| later_trimmed.strip_prefix("> "))
            .or_else(|| later_trimmed.strip_prefix("# "))
        else {
            continue;
        };

        let later_indent_len = later.len().saturating_sub(later_trimmed.len());
        let later_indent = &later[..later_indent_len];
        *later = format!("{}{}", later_indent, rest);
    }
}

fn looks_like_markdown_heading(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("# ") else {
        return false;
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return false;
    }

    // Very conservative: headings are wordy and lack shell operators.
    !rest.contains('|')
        && !rest.contains('&')
        && !rest.contains('>')
        && !rest.contains('<')
        && !rest.contains('\\')
        && !rest.contains('=')
        && !rest.contains(';')
        && !rest.contains("--")
}

fn looks_command_like(s: &str, aggressiveness: Aggressiveness) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }

    let has_operator = s.contains('|')
        || s.contains("&&")
        || s.contains(';')
        || s.contains('>')
        || s.contains('<')
        || s.contains("$(")
        || s.contains('`');

    let has_flag_or_path =
        s.contains("--") || s.contains(" -") || s.contains('/') || s.contains("./");

    let word_count = s.split_whitespace().count();

    match aggressiveness {
        Aggressiveness::Low => has_operator || s.ends_with('\\'),
        Aggressiveness::Normal => {
            has_operator || has_flag_or_path || s.ends_with('\\') || word_count >= 2
        }
        Aggressiveness::High => true,
    }
}

fn flatten_dropping_blank_lines(lines: &[String]) -> String {
    let mut group: Vec<&str> = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        group.push(line);
    }
    flatten_group(&group)
}

fn flatten_preserving_blank_lines(lines: &[String]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut group: Vec<&str> = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            if !group.is_empty() {
                out.push(flatten_group(&group));
                group.clear();
            }
            out.push(String::new());
            continue;
        }
        group.push(line);
    }

    if !group.is_empty() {
        out.push(flatten_group(&group));
    }

    out.join("\n")
}

fn flatten_group(group: &[&str]) -> String {
    let mut out = String::new();

    for raw_line in group {
        let mut s = raw_line.trim();
        if s.is_empty() {
            continue;
        }

        let continued = s.ends_with('\\');
        if continued {
            s = s.trim_end_matches('\\').trim_end();
        }

        if s.is_empty() {
            continue;
        }

        if !out.is_empty() {
            out.push(' ');
        }

        out.push_str(s);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_backslash_continuations() {
        let input = "kubectl get pods \\\n  -n kube-system \\\n  | jq '.items[].metadata.name'\n";

        let res = trim_text(input, &TrimOptions::default());
        assert_eq!(
            res.trimmed,
            "kubectl get pods -n kube-system | jq '.items[].metadata.name'"
        );
        assert!(res.changed);
    }

    #[test]
    fn strips_box_drawing_gutters() {
        let input = "  ┃  hello\n┃  world  \n";
        let res = trim_text(input, &TrimOptions::default());
        assert_eq!(res.trimmed, "hello world");
    }

    #[test]
    fn keeps_markdown_heading() {
        let input = "# Release Notes\n";
        let res = trim_text(input, &TrimOptions::default());
        assert_eq!(res.trimmed, "# Release Notes");
    }

    #[test]
    fn strips_shell_prompt_on_commands() {
        let input = "$ brew install foo\n";
        let res = trim_text(input, &TrimOptions::default());
        assert_eq!(res.trimmed, "brew install foo");
    }
}
