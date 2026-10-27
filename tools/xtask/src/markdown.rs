//! Repository-local Markdown destinations and GitHub-style heading anchors.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use percent_encoding::percent_decode_str;
use regex::Regex;
use crate::process::Result;

/// Shared expressions for the maintained Markdown subset, excluding fenced code.
struct Syntax {
    /// Inline links and images, optionally followed by a title.
    inline: Regex,
    /// Link reference definitions.
    reference: Regex,
    /// HTML href/src attributes.
    html: Regex,
    /// ATX headings with at most three leading spaces.
    heading: Regex,
    /// Source file line fragments do not name Markdown headings.
    line: Regex,
}
impl Syntax {
    /// Compile static patterns once per check rather than once per destination.
    fn new() -> Self {
        Self {
            inline: Regex::new(r"!?\[[^\]]*\]\((?P<target><[^>]+>|[^\s)]+)(?:\s+[^)]*)?\)").unwrap(),
            reference: Regex::new(r"^\s*\[[^\]]+\]:\s*(?P<target><[^>]+>|\S+)").unwrap(),
            html: Regex::new(r#"\b(?:href|src)=["'](?P<target>[^"']+)["']"#).unwrap(),
            heading: Regex::new(r"^ {0,3}#{1,6}\s+(.*?)\s*#*\s*$").unwrap(),
            line: Regex::new(r"^L\d+(?:-L\d+)?$").unwrap(),
        }
    }

    /// Normalize formatted headings and retain Unicode letters/digits like GitHub.
    fn anchor(&self, text: &str) -> String {
        let tags = Regex::new(r"<[^>]*>").unwrap();
        let labels = Regex::new(r"!?\[([^\]]+)\]\([^)]*\)").unwrap();
        let plain = tags.replace_all(text, "");
        let plain = labels.replace_all(&plain, "$1");
        let filtered: String = plain
            .chars()
            .filter(|c| !matches!(c, '`' | '*' | '_'))
            .flat_map(char::to_lowercase)
            .filter(|c| c.is_alphanumeric() || matches!(c, '-' | ' '))
            .collect();
        filtered.split_whitespace().collect::<Vec<_>>().join("-")
    }

    /// Heading duplicates use the same monotonically numbered suffix as GitHub.
    fn anchors(&self, path: &Path) -> Result<BTreeSet<String>> {
        let mut occurrences = BTreeMap::<String, usize>::new();
        let mut result = BTreeSet::new();
        let mut fenced = false;
        for line in fs::read_to_string(path)?.lines() {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
                continue;
            }
            if fenced {
                continue;
            }
            if let Some(heading) = self.heading.captures(line) {
                let base = self.anchor(&heading[1]);
                let count = occurrences.entry(base.clone()).or_default();
                result.insert(if *count == 0 { base } else { format!("{base}-{count}") });
                *count += 1;
            }
        }
        Ok(result)
    }

    /// Validate a local file or anchor; external network resources are not fetched.
    fn validate(&self, source: &Path, target: &str) -> Result<bool> {
        let target = target.trim_start_matches('<').trim_end_matches('>');
        if target.starts_with("//") {
            return Ok(false);
        }
        if let Some((scheme, _)) = target.split_once(':') {
            if !scheme.contains(['/', '#', '?']) {
                if matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "mailto" | "tel") {
                    return Ok(false);
                }
                return Err(format!("unsupported link scheme: {target}").into());
            }
        }
        let (path, fragment) = target.split_once('#').unwrap_or((target, ""));
        let path = path.split('?').next().unwrap();
        let path = percent_decode_str(path).decode_utf8()?;
        let destination = if path.is_empty() {
            source.to_path_buf()
        } else {
            source.parent().unwrap().join(path.as_ref())
        };
        if !destination.exists() {
            return Err(format!("missing target {target:?}: {}", destination.display()).into());
        }
        let fragment = percent_decode_str(fragment).decode_utf8()?;
        if !fragment.is_empty()
            && destination.extension().is_some_and(|e| e == "md")
            && !self.line.is_match(&fragment)
            && !self.anchors(&destination)?.contains(fragment.as_ref())
        {
            return Err(format!("missing Markdown anchor #{fragment} in {}", destination.display()).into());
        }
        Ok(true)
    }
}

/// Visit product Markdown only; private plans/reviews are not repository checks.
fn inventory(root: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if matches!(entry.file_name().to_str(), Some("target" | ".git" | "__pycache__" | "plans" | "reviews")) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            inventory(&entry.path(), files)?;
        } else if entry.path().extension().is_some_and(|e| e == "md") {
            files.push(entry.path());
        }
    }
    Ok(())
}

/// Check all maintained Markdown roots, reporting every bad destination together.
pub(crate) fn check(root: &Path) -> Result<()> {
    let syntax = Syntax::new();
    let mut files = Vec::new();
    for name in ["README.md", "CHANGELOG.md"] {
        let path = root.join(name);
        if path.is_file() {
            files.push(path);
        }
    }
    for name in ["crates", "tools", "examples", "docs", "schemas", "tests"] {
        inventory(&root.join(name), &mut files)?;
    }
    files.sort();
    let mut checked = 0;
    let mut errors = Vec::new();
    for source in &files {
        let mut fenced = false;
        for (index, line) in fs::read_to_string(source)?.lines().enumerate() {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
                continue;
            }
            if fenced {
                continue;
            }
            for pattern in [&syntax.inline, &syntax.html, &syntax.reference] {
                for captures in pattern.captures_iter(line) {
                    match syntax.validate(source, &captures["target"]) {
                        Ok(local) => checked += usize::from(local),
                        Err(error) => errors.push(format!("{}:{}: {error}", source.strip_prefix(root)?.display(), index + 1)),
                    }
                }
            }
        }
    }
    if !errors.is_empty() {
        return Err(format!("Markdown link check failed:\n{}", errors.join("\n")).into());
    }
    println!("checked {checked} repository-local links across {} Markdown files", files.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Encoded paths, duplicate headings, HTML and source-line links stay usable.
    #[test]
    fn local_paths_and_duplicate_anchors_are_checked() -> Result<()> {
        let root = tempfile::tempdir()?;
        let source = root.path().join("source.md");
        fs::write(&source, "# A `value`\n# A `value`\n# Héllo World\n```\n# hidden\n```\n")?;
        let destination = root.path().join("file name.md");
        fs::copy(&source, &destination)?;
        let syntax = Syntax::new();
        for valid in ["#a-value", "#a-value-1", "#héllo-world", "file%20name.md#a-value", "<file%20name.md#L4-L9>"] {
            assert!(syntax.validate(&source, valid)?);
        }
        for invalid in ["#a-value-2", "#hidden", "absent.md", "unknown:foo"] {
            assert!(syntax.validate(&source, invalid).is_err(), "{invalid}");
        }
        assert!(!syntax.validate(&source, "https://example.com/no-lookup")?);
        Ok(())
    }
}
