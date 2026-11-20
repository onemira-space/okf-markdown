//! Zero-dependency Rust library for **Open Knowledge Format (`OKF v0.2`)** Markdown vaults.
//!
//! `okf-markdown` parses YAML-subset frontmatter, validates required OKF node fields,
//! extracts deterministic `[[wiki-links]]`, relative Markdown links, and `- [ ]` / `- [x]`
//! `@owner` action items, builds relational vault graphs, and converts WebVTT / SRT /
//! speaker-labeled transcripts into deterministic OKF Markdown session notes.
//!
//! ## Ecosystem & Documentation
//!
//! - **Official MCP Integration Docs**: <https://onemira.com/docs/mcp-integration>
//! - **Local Wiki Architecture**: <https://onemira.com/docs/daily-use/local-wiki>
//! - **Ready-to-Use OKF Meeting Templates**: <https://onemira.com/meeting-minutes-templates>
//! - **Standalone Local-AI Capture Hardware**: [OneMira C1](https://onemira.com/products/c1)
//!   (`$0/mo` local AI, 99-language offline transcription, Speaker ID, Zero Cloud)
//! - **OneMira Desktop (macOS & Windows)**: <https://onemira.com/download>
//! - **No-Subscription Local AI Architecture**: <https://onemira.com/ai-voice-recorder-without-subscription>
//!
//! ## Quick Example
//!
//! ```rust
//! use okf_markdown::{validate_okf_document, extract_action_items};
//!
//! let doc = "\
//! ---
//! okf_version: \"0.2\"
//! id: \"session/2026-11-20-architecture-review\"
//! type: \"meeting\"
//! title: \"Architecture Review\"
//! date: \"2026-11-20\"
//! speakers: [\"Maya Lin\", \"Alex Rivera\"]
//! ---
//!
//! # Architecture Review
//!
//! See [[topics/local-wiki]] for storage invariants.
//!
//! ## Action Items
//! - [ ] @alex Add OKF schema check to CI
//! - [x] @maya Publish Rust crate
//! ";
//!
//! let report = validate_okf_document(doc, "sessions/review.md");
//! assert!(report.valid);
//! assert_eq!(report.links.wiki_links.len(), 1);
//! assert_eq!(report.links.wiki_links[0].target, "topics/local-wiki");
//!
//! let actions = extract_action_items(doc);
//! assert_eq!(actions.len(), 2);
//! assert_eq!(actions[0].owner.as_deref(), Some("alex"));
//! assert!(!actions[0].completed);
//! assert!(actions[1].completed);
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Current Open Knowledge Format (OKF) specification version supported by this crate.
pub const OKF_SPEC_VERSION: &str = "0.2";

/// Canonical OKF v0.2 node `type` values.
pub const VALID_NODE_TYPES: &[&str] = &[
    "session",
    "meeting",
    "topic",
    "person",
    "project",
    "decision",
    "note",
];

/// Typed scalar or string-list value parsed from OKF YAML frontmatter.
#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterValue {
    /// UTF-8 string value.
    Str(String),
    /// Boolean value (`true` or `false`).
    Bool(bool),
    /// Numeric value.
    Number(f64),
    /// Ordered list of string items (inline `[a, b]` or block `- item`).
    List(Vec<String>),
}

impl FrontmatterValue {
    /// Returns the string slice if this value is [`FrontmatterValue::Str`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FrontmatterValue::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns the list slice if this value is [`FrontmatterValue::List`].
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            FrontmatterValue::List(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}

/// Result of parsing a Markdown document's YAML frontmatter block.
#[derive(Debug, Clone, PartialEq)]
pub struct FrontmatterParseResult {
    /// Parsed frontmatter key-value map (`None` if frontmatter delimiters are missing or unclosed).
    pub frontmatter: Option<BTreeMap<String, FrontmatterValue>>,
    /// Markdown body following the closing `---` delimiter.
    pub body: String,
    /// Structural parse errors encountered while reading frontmatter.
    pub errors: Vec<String>,
}

/// A deterministic `[[target#heading|alias]]` wiki-link extracted from Markdown prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// Normalized target slug/path before `#` and `|`.
    pub target: String,
    /// Optional heading anchor after `#`.
    pub heading: Option<String>,
    /// Optional display alias after `|`.
    pub alias: Option<String>,
}

/// A relative Markdown link `[label](./path.md)` (excluding external URLs and images).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativeMarkdownLink {
    /// Visible link label.
    pub label: String,
    /// Relative target path without `#fragment`.
    pub href: String,
}

/// All internal links extracted from a Markdown body (ignoring code blocks and inline code).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractedLinks {
    /// Extracted `[[wiki-links]]`.
    pub wiki_links: Vec<WikiLink>,
    /// Extracted relative Markdown links.
    pub markdown_links: Vec<RelativeMarkdownLink>,
}

/// A Markdown task checkbox item (`- [ ]` or `- [x]`) with optional `@owner`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionItem {
    /// 1-based line number in the source Markdown.
    pub line: usize,
    /// `true` for `- [x]` / `- [X]`, `false` for `- [ ]`.
    pub completed: bool,
    /// First `@handle` owner mention found on the line, if any.
    pub owner: Option<String>,
    /// Full task text following the checkbox marker.
    pub text: String,
}

/// Full schema & link validation report for a single OKF v0.2 document.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    /// Logical or filesystem path of the validated document.
    pub file_path: String,
    /// `true` when `errors` is empty.
    pub valid: bool,
    /// Hard schema or syntax errors.
    pub errors: Vec<String>,
    /// Non-fatal recommendations (e.g., missing `okf_version`).
    pub warnings: Vec<String>,
    /// Parsed frontmatter map, if valid delimiters were present.
    pub frontmatter: Option<BTreeMap<String, FrontmatterValue>>,
    /// Extracted internal wiki and relative Markdown links.
    pub links: ExtractedLinks,
}

/// A directed edge between two notes in an OKF vault knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    /// Source note relative path.
    pub from: String,
    /// Resolved target relative path, or raw target string if unresolved.
    pub to: String,
    /// `true` when the target resolved to an existing `.md` note in the vault.
    pub resolved: bool,
}

/// Summary statistics for a scanned OKF Markdown vault.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VaultStats {
    /// Total `.md` files scanned.
    pub files: usize,
    /// Total resolved + unresolved edges.
    pub edges: usize,
    /// Total schema validation errors across all files.
    pub schema_errors: usize,
    /// Number of unresolved internal links.
    pub broken_links: usize,
    /// Number of notes with zero incoming and zero outgoing links.
    pub orphans: usize,
    /// Number of open `- [ ]` action items.
    pub open_actions: usize,
    /// Number of completed `- [x]` action items.
    pub completed_actions: usize,
}

/// Complete knowledge graph and validation diagnostics for a vault directory.
#[derive(Debug, Clone, PartialEq)]
pub struct VaultGraph {
    /// Per-file validation reports sorted by relative path.
    pub documents: Vec<ValidationResult>,
    /// Directed internal link edges.
    pub edges: Vec<GraphEdge>,
    /// Unresolved `(source_file, target)` pairs.
    pub broken_links: Vec<(String, String)>,
    /// Relative paths of notes with no incoming or outgoing edges.
    pub orphans: Vec<String>,
    /// Aggregate vault counters.
    pub stats: VaultStats,
}

/// Output of converting a raw WebVTT, SRT, or speaker-labeled transcript into OKF v0.2 Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertedTranscript {
    /// Rendered OKF v0.2 Markdown document ready to write to disk.
    pub markdown: String,
    /// Distinct speakers detected in order of appearance.
    pub speakers: Vec<String>,
    /// Number of spoken turns emitted.
    pub turn_count: usize,
}

fn unquote(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].replace("\\\"", "\"")
    } else {
        trimmed.to_string()
    }
}

fn split_inline_array(inner: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut buf = String::new();
    let mut quote: Option<char> = None;
    let mut prev_escape = false;

    for ch in inner.chars() {
        if prev_escape {
            buf.push(ch);
            prev_escape = false;
            continue;
        }
        if ch == '\\' && quote == Some('"') {
            buf.push(ch);
            prev_escape = true;
            continue;
        }
        match quote {
            Some(q) if ch == q => {
                buf.push(ch);
                quote = None;
            }
            None if ch == '"' || ch == '\'' => {
                buf.push(ch);
                quote = Some(ch);
            }
            None if ch == ',' => {
                let item = unquote(&buf);
                if !item.is_empty() {
                    items.push(item);
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }
    let tail = unquote(&buf);
    if !tail.is_empty() {
        items.push(tail);
    }
    items
}

fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Parse zero-dependency YAML-subset frontmatter (scalars, quoted strings, inline `[a, b]` arrays,
/// and block `- item` arrays) from an OKF Markdown string.
pub fn parse_frontmatter(markdown: &str) -> FrontmatterParseResult {
    let normalized = markdown.replace("\r\n", "\n");
    if !normalized.starts_with("---\n") {
        return FrontmatterParseResult {
            frontmatter: None,
            body: normalized,
            errors: vec!["Missing YAML frontmatter delimiter (---)".to_string()],
        };
    }

    let rest = &normalized[4..];
    let (raw_yaml, body) = if let Some(idx) = rest.find("\n---\n") {
        (&rest[..idx], rest[idx + 5..].to_string())
    } else if let Some(stripped) = rest.strip_suffix("\n---") {
        (stripped, String::new())
    } else {
        return FrontmatterParseResult {
            frontmatter: None,
            body: normalized,
            errors: vec!["Unclosed YAML frontmatter block".to_string()],
        };
    };

    let mut frontmatter = BTreeMap::new();
    let mut errors = Vec::new();
    let mut current_array_key: Option<String> = None;

    for raw_line in raw_yaml.split('\n') {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(after_dash) = trimmed.strip_prefix("- ") {
            if let Some(ref key) = current_array_key {
                if let Some(FrontmatterValue::List(vec)) = frontmatter.get_mut(key) {
                    vec.push(unquote(after_dash));
                    continue;
                }
            }
        }

        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim();
            let raw_val = line[colon + 1..].trim();
            if is_valid_key(key) {
                current_array_key = None;
                if raw_val.is_empty() {
                    frontmatter.insert(key.to_string(), FrontmatterValue::List(Vec::new()));
                    current_array_key = Some(key.to_string());
                } else if raw_val.starts_with('[') && raw_val.ends_with(']') {
                    let inner = raw_val[1..raw_val.len() - 1].trim();
                    frontmatter.insert(
                        key.to_string(),
                        FrontmatterValue::List(split_inline_array(inner)),
                    );
                } else if raw_val == "true" {
                    frontmatter.insert(key.to_string(), FrontmatterValue::Bool(true));
                } else if raw_val == "false" {
                    frontmatter.insert(key.to_string(), FrontmatterValue::Bool(false));
                } else if let Ok(num) = raw_val.parse::<f64>() {
                    if raw_val.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '.') {
                        frontmatter.insert(key.to_string(), FrontmatterValue::Number(num));
                    } else {
                        frontmatter.insert(key.to_string(), FrontmatterValue::Str(unquote(raw_val)));
                    }
                } else {
                    frontmatter.insert(key.to_string(), FrontmatterValue::Str(unquote(raw_val)));
                }
                continue;
            }
        }

        errors.push(format!("Invalid frontmatter line: {line}"));
    }

    FrontmatterParseResult {
        frontmatter: Some(frontmatter),
        body,
        errors,
    }
}

fn strip_code_spans(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_fence = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            out.push('\n');
            continue;
        }
        if in_fence {
            out.push('\n');
            continue;
        }
        let mut in_inline = false;
        for ch in line.chars() {
            if ch == '`' {
                in_inline = !in_inline;
            } else if !in_inline {
                out.push(ch);
            }
        }
        out.push('\n');
    }
    out
}

/// Extract `[[wiki-links]]` and relative Markdown links `[label](./file.md)` while ignoring
/// fenced code blocks and inline code spans.
pub fn extract_links(body: &str) -> ExtractedLinks {
    let cleaned = strip_code_spans(body);
    let bytes = cleaned.as_bytes();
    let mut wiki_links = Vec::new();
    let mut markdown_links = Vec::new();

    // 1. Scan [[target#heading|alias]]
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(rel_end) = cleaned[i + 2..].find("]]") {
                let inner = &cleaned[i + 2..i + 2 + rel_end];
                if !inner.contains('\n') && !inner.trim().is_empty() {
                    let (left, alias) = match inner.split_once('|') {
                        Some((l, a)) => (l, Some(a.trim().to_string()).filter(|s| !s.is_empty())),
                        None => (inner, None),
                    };
                    let (target_raw, heading) = match left.split_once('#') {
                        Some((t, h)) => (t, Some(h.trim().to_string()).filter(|s| !s.is_empty())),
                        None => (left, None),
                    };
                    let target = target_raw.trim().to_string();
                    if !target.is_empty() {
                        wiki_links.push(WikiLink {
                            target,
                            heading,
                            alias,
                        });
                    }
                }
                i += 4 + rel_end;
                continue;
            }
        }
        i += 1;
    }

    // 2. Scan relative [label](href) (skipping ![alt](img) and [[wiki]])
    let mut j = 0;
    while j < bytes.len() {
        if bytes[j] == b'['
            && (j == 0 || (bytes[j - 1] != b'!' && bytes[j - 1] != b'['))
            && (j + 1 >= bytes.len() || bytes[j + 1] != b'[')
        {
            if let Some(close_bracket) = cleaned[j + 1..].find(']') {
                let label_end = j + 1 + close_bracket;
                if label_end + 1 < bytes.len() && bytes[label_end + 1] == b'(' {
                    if let Some(close_paren) = cleaned[label_end + 2..].find(')') {
                        let label = cleaned[j + 1..label_end].trim();
                        let raw_href = cleaned[label_end + 2..label_end + 2 + close_paren].trim();
                        let href = raw_href.split('#').next().unwrap_or("").trim();
                        let lower = href.to_ascii_lowercase();
                        let is_external = lower.starts_with("http://")
                            || lower.starts_with("https://")
                            || lower.starts_with("mailto:")
                            || lower.starts_with("tel:");
                        if !href.is_empty() && !is_external && !label.contains('\n') {
                            markdown_links.push(RelativeMarkdownLink {
                                label: label.to_string(),
                                href: href.to_string(),
                            });
                        }
                        j = label_end + 3 + close_paren;
                        continue;
                    }
                }
            }
        }
        j += 1;
    }

    ExtractedLinks {
        wiki_links,
        markdown_links,
    }
}

/// Extract `- [ ]` and `- [x]` Markdown action items and optional `@owner` handles.
pub fn extract_action_items(markdown: &str) -> Vec<ActionItem> {
    let mut items = Vec::new();
    let mut in_fence = false;

    for (idx, raw_line) in markdown.lines().enumerate() {
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let rest = if let Some(r) = trimmed.strip_prefix("- [ ] ") {
            Some((false, r))
        } else if let Some(r) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
        {
            Some((true, r))
        } else {
            None
        };

        if let Some((completed, text)) = rest {
            let owner = text.split_whitespace().find_map(|tok| {
                let handle = tok.strip_prefix('@')?;
                let clean: String = handle
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                if clean.is_empty() {
                    None
                } else {
                    Some(clean)
                }
            });
            items.push(ActionItem {
                line: idx + 1,
                completed,
                owner,
                text: text.trim().to_string(),
            });
        }
    }
    items
}

fn is_valid_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 10 {
        return false;
    }
    b[0..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
}

/// Validate a single OKF v0.2 Markdown document and extract its internal links.
pub fn validate_okf_document(markdown: &str, file_path: &str) -> ValidationResult {
    let parsed = parse_frontmatter(markdown);
    let mut errors = parsed.errors;
    let mut warnings = Vec::new();

    if let Some(ref fm) = parsed.frontmatter {
        if !fm.contains_key("okf_version") {
            warnings.push("Missing recommended `okf_version: \"0.2\"` field".to_string());
        }
        match fm.get("id").and_then(FrontmatterValue::as_str) {
            Some(s) if !s.trim().is_empty() => {}
            _ => errors.push("Missing required string field `id`".to_string()),
        }
        match fm.get("title").and_then(FrontmatterValue::as_str) {
            Some(s) if !s.trim().is_empty() => {}
            _ => errors.push("Missing required string field `title`".to_string()),
        }
        match fm.get("type").and_then(FrontmatterValue::as_str) {
            Some(t) if VALID_NODE_TYPES.contains(&t) => {}
            _ => errors.push(format!(
                "Field `type` must be one of: {}",
                VALID_NODE_TYPES.join(", ")
            )),
        }
        for date_key in ["created_at", "date"] {
            if let Some(val) = fm.get(date_key).and_then(FrontmatterValue::as_str) {
                if !is_valid_iso_date(val) {
                    errors.push(format!(
                        "Field `{date_key}` is not a valid ISO-8601 date: {val}"
                    ));
                }
            }
        }
    }

    let links = extract_links(&parsed.body);
    let valid = errors.is_empty();
    ValidationResult {
        file_path: file_path.to_string(),
        valid,
        errors,
        warnings,
        frontmatter: parsed.frontmatter,
        links,
    }
}

fn collect_markdown_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }
        if path.is_dir() {
            collect_markdown_files(root, &path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(())
}

fn normalize_slug(rel_path: &str) -> String {
    rel_path
        .replace('\\', "/")
        .trim_start_matches("./")
        .strip_suffix(".md")
        .unwrap_or(rel_path)
        .to_ascii_lowercase()
}

/// Scan an OKF Markdown vault directory, validate every `.md` file, resolve `[[wiki-links]]`
/// and relative Markdown links, and return the full graph + aggregate stats.
pub fn build_vault_graph(vault_dir: &Path) -> io::Result<VaultGraph> {
    let mut files = Vec::new();
    collect_markdown_files(vault_dir, vault_dir, &mut files)?;
    files.sort();

    let mut documents = Vec::with_capacity(files.len());
    let mut slug_to_rel: BTreeMap<String, String> = BTreeMap::new();
    let mut basename_to_rel: BTreeMap<String, String> = BTreeMap::new();
    let mut open_actions = 0usize;
    let mut completed_actions = 0usize;

    for abs in &files {
        let rel = abs
            .strip_prefix(vault_dir)
            .unwrap_or(abs)
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(abs)?;
        for item in extract_action_items(&content) {
            if item.completed {
                completed_actions += 1;
            } else {
                open_actions += 1;
            }
        }
        let doc = validate_okf_document(&content, &rel);
        let slug = normalize_slug(&rel);
        slug_to_rel.insert(slug.clone(), rel.clone());
        if let Some(base) = slug.rsplit('/').next() {
            basename_to_rel.entry(base.to_string()).or_insert_with(|| rel.clone());
        }
        if let Some(id) = doc
            .frontmatter
            .as_ref()
            .and_then(|fm| fm.get("id"))
            .and_then(FrontmatterValue::as_str)
        {
            slug_to_rel.insert(id.to_ascii_lowercase(), rel.clone());
        }
        documents.push(doc);
    }

    let mut edges = Vec::new();
    let mut broken_links = Vec::new();
    let mut connected: BTreeSet<String> = BTreeSet::new();

    for doc in &documents {
        for wl in &doc.links.wiki_links {
            let key = normalize_slug(&wl.target);
            let target_rel = slug_to_rel
                .get(&key)
                .or_else(|| basename_to_rel.get(&key))
                .cloned();
            match target_rel {
                Some(resolved_path) => {
                    connected.insert(doc.file_path.clone());
                    connected.insert(resolved_path.clone());
                    edges.push(GraphEdge {
                        from: doc.file_path.clone(),
                        to: resolved_path,
                        resolved: true,
                    });
                }
                None => {
                    broken_links.push((doc.file_path.clone(), wl.target.clone()));
                    edges.push(GraphEdge {
                        from: doc.file_path.clone(),
                        to: wl.target.clone(),
                        resolved: false,
                    });
                }
            }
        }

        let parent = Path::new(&doc.file_path)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        for ml in &doc.links.markdown_links {
            let joined = parent.join(&ml.href);
            let norm = normalize_slug(&joined.to_string_lossy());
            match slug_to_rel.get(&norm).cloned() {
                Some(resolved_path) => {
                    connected.insert(doc.file_path.clone());
                    connected.insert(resolved_path.clone());
                    edges.push(GraphEdge {
                        from: doc.file_path.clone(),
                        to: resolved_path,
                        resolved: true,
                    });
                }
                None => {
                    broken_links.push((doc.file_path.clone(), ml.href.clone()));
                    edges.push(GraphEdge {
                        from: doc.file_path.clone(),
                        to: ml.href.clone(),
                        resolved: false,
                    });
                }
            }
        }
    }

    let orphans: Vec<String> = documents
        .iter()
        .filter(|d| !connected.contains(&d.file_path))
        .map(|d| d.file_path.clone())
        .collect();
    let schema_errors: usize = documents.iter().map(|d| d.errors.len()).sum();

    let stats = VaultStats {
        files: documents.len(),
        edges: edges.len(),
        schema_errors,
        broken_links: broken_links.len(),
        orphans: orphans.len(),
        open_actions,
        completed_actions,
    };

    Ok(VaultGraph {
        documents,
        edges,
        broken_links,
        orphans,
        stats,
    })
}

/// Convert a WebVTT, SRT, or `Speaker: text` transcript into a schema-valid OKF v0.2 Markdown note.
pub fn convert_transcript_to_okf(
    raw: &str,
    title: &str,
    id: &str,
    date: &str,
) -> ConvertedTranscript {
    let mut speakers = Vec::new();
    let mut speaker_seen = BTreeSet::new();
    let mut turns: Vec<(Option<String>, Option<String>, String)> = Vec::new();
    let mut current_ts: Option<String> = None;

    for raw_line in raw.replace("\r\n", "\n").lines() {
        let line = raw_line.trim();
        if line.is_empty()
            || line == "WEBVTT"
            || line.starts_with("NOTE")
            || line.chars().all(|c| c.is_ascii_digit())
        {
            continue;
        }
        if line.contains("-->") {
            let start = line.split("-->").next().unwrap_or("").trim();
            let clean_ts = start.split('.').next().unwrap_or(start).split(',').next().unwrap_or(start);
            current_ts = Some(clean_ts.to_string());
            continue;
        }

        // Handle `<v Speaker>text` (WebVTT voice tag) or `Speaker: text`
        let (speaker, text) = if let Some(after_v) = line.strip_prefix("<v ") {
            if let Some((sp, rest)) = after_v.split_once('>') {
                let cleaned = rest.replace("</v>", "").trim().to_string();
                (Some(sp.trim().to_string()), cleaned)
            } else {
                (None, line.to_string())
            }
        } else if let Some((sp, rest)) = line.split_once(':') {
            let sp_t = sp.trim();
            if !sp_t.is_empty()
                && sp_t.len() <= 40
                && !sp_t.contains("http")
                && !sp_t.chars().all(|c| c.is_ascii_digit())
            {
                (Some(sp_t.to_string()), rest.trim().to_string())
            } else {
                (None, line.to_string())
            }
        } else {
            (None, line.to_string())
        };

        if let Some(ref sp) = speaker {
            if speaker_seen.insert(sp.clone()) {
                speakers.push(sp.clone());
            }
        }
        if !text.is_empty() {
            turns.push((current_ts.clone(), speaker, text));
        }
    }

    let speakers_yaml = speakers
        .iter()
        .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");

    let mut md = String::new();
    md.push_str("---\n");
    md.push_str(&format!("okf_version: \"{OKF_SPEC_VERSION}\"\n"));
    md.push_str(&format!("id: \"{}\"\n", id.replace('"', "\\\"")));
    md.push_str("type: \"session\"\n");
    md.push_str(&format!("title: \"{}\"\n", title.replace('"', "\\\"")));
    md.push_str(&format!("date: \"{date}\"\n"));
    md.push_str(&format!("speakers: [{speakers_yaml}]\n"));
    md.push_str("---\n\n");
    md.push_str(&format!("# {title}\n\n## Transcript\n\n"));

    for (ts, sp, text) in &turns {
        match (ts, sp) {
            (Some(t), Some(s)) => md.push_str(&format!("- **{t} — {s}**: {text}\n")),
            (None, Some(s)) => md.push_str(&format!("- **{s}**: {text}\n")),
            (Some(t), None) => md.push_str(&format!("- **{t}**: {text}\n")),
            (None, None) => md.push_str(&format!("- {text}\n")),
        }
    }

    ConvertedTranscript {
        markdown: md,
        speakers,
        turn_count: turns.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DOC: &str = "\
---
okf_version: \"0.2\"
id: \"session/2026-11-20-sync\"
type: \"meeting\"
title: \"Sync Protocol Review\"
date: \"2026-11-20\"
speakers: [\"Maya Lin\", \"Alex Rivera\"]
tags:
  - architecture
  - local-first
---

# Sync Protocol Review

Referenced [[topics/local-wiki#invariants|Local Wiki]] and [Design Doc](./design.md).
Ignored code link: `[[not/a/link]]` and [ext](https://onemira.com/docs/mcp-integration).

## Action Items
- [ ] @alex Add schema lint to pre-commit
- [x] @maya Draft OKF v0.2 spec
";

    #[test]
    fn parses_inline_and_block_yaml_arrays() {
        let parsed = parse_frontmatter(SAMPLE_DOC);
        assert!(parsed.errors.is_empty());
        let fm = parsed.frontmatter.expect("frontmatter present");
        assert_eq!(fm.get("okf_version").and_then(FrontmatterValue::as_str), Some("0.2"));
        assert_eq!(
            fm.get("speakers").and_then(FrontmatterValue::as_list),
            Some(&["Maya Lin".to_string(), "Alex Rivera".to_string()][..])
        );
        assert_eq!(
            fm.get("tags").and_then(FrontmatterValue::as_list),
            Some(&["architecture".to_string(), "local-first".to_string()][..])
        );
    }

    #[test]
    fn validates_okf_document_and_links() {
        let report = validate_okf_document(SAMPLE_DOC, "sessions/sync.md");
        assert!(report.valid, "errors: {:?}", report.errors);
        assert!(report.warnings.is_empty());
        assert_eq!(report.links.wiki_links.len(), 1);
        assert_eq!(report.links.wiki_links[0].target, "topics/local-wiki");
        assert_eq!(report.links.wiki_links[0].heading.as_deref(), Some("invariants"));
        assert_eq!(report.links.wiki_links[0].alias.as_deref(), Some("Local Wiki"));
        assert_eq!(report.links.markdown_links.len(), 1);
        assert_eq!(report.links.markdown_links[0].href, "./design.md");
    }

    #[test]
    fn rejects_invalid_node_type_and_bad_date() {
        let bad = "---\nid: \"x\"\ntitle: \"Bad\"\ntype: \"unknown\"\ndate: \"2026/11/20\"\n---\nBody\n";
        let report = validate_okf_document(bad, "bad.md");
        assert!(!report.valid);
        assert_eq!(report.errors.len(), 2);
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn extracts_action_items_and_owners() {
        let actions = extract_action_items(SAMPLE_DOC);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].owner.as_deref(), Some("alex"));
        assert!(!actions[0].completed);
        assert_eq!(actions[1].owner.as_deref(), Some("maya"));
        assert!(actions[1].completed);
    }

    #[test]
    fn converts_webvtt_to_valid_okf_markdown() {
        let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:04.000\n<v Maya Lin>Keep notes in plain Markdown.</v>\n\n00:00:05.000 --> 00:00:08.000\nAlex Rivera: Agreed, zero cloud lock-in.\n";
        let out = convert_transcript_to_okf(vtt, "Kickoff", "session/kickoff", "2026-11-20");
        assert_eq!(out.turn_count, 2);
        assert_eq!(out.speakers, vec!["Maya Lin", "Alex Rivera"]);
        let check = validate_okf_document(&out.markdown, "kickoff.md");
        assert!(check.valid, "converted markdown invalid: {:?}", check.errors);
    }

    #[test]
    fn builds_vault_graph_with_resolved_and_broken_edges() {
        let base = std::env::temp_dir().join(format!("okf_test_vault_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("topics")).unwrap();
        fs::write(
            base.join("topics/local-wiki.md"),
            "---\nokf_version: \"0.2\"\nid: \"topics/local-wiki\"\ntype: \"topic\"\ntitle: \"Local Wiki\"\n---\nCore topic.\n",
        )
        .unwrap();
        fs::write(
            base.join("session.md"),
            "---\nokf_version: \"0.2\"\nid: \"session/1\"\ntype: \"session\"\ntitle: \"S1\"\n---\nLinks [[topics/local-wiki]] and [[missing/note]].\n- [ ] @alex Verify graph\n",
        )
        .unwrap();

        let graph = build_vault_graph(&base).unwrap();
        assert_eq!(graph.stats.files, 2);
        assert_eq!(graph.stats.schema_errors, 0);
        assert_eq!(graph.stats.edges, 2);
        assert_eq!(graph.stats.broken_links, 1);
        assert_eq!(graph.stats.orphans, 0);
        assert_eq!(graph.stats.open_actions, 1);
        let _ = fs::remove_dir_all(&base);
    }
}
