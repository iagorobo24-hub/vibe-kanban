//! Design Mode data contracts and service for AgentOS.
//!
//! A design system lives in `{worktree}/design-system/MASTER.md` and is parsed
//! into [`DesignSystemTokens`]. Generation delegates to the universal Python
//! skill script; auditing scans TSX/JSX/HTML for basic design-rule violations.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct UIStyleInfo {
    pub name: String,
    pub keywords: Option<String>,
    pub best_for: Option<String>,
    pub performance: Option<String>,
    pub accessibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ColorPaletteInfo {
    pub primary: String,
    pub secondary: String,
    pub cta: String,
    pub background: String,
    pub text: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct TypographyInfo {
    pub heading: String,
    pub body: String,
    pub mood: Option<String>,
    pub best_for: Option<String>,
    pub google_fonts: Option<String>,
    pub css_import: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ChecklistItem {
    pub rule: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct DesignSystemTokens {
    pub project_name: String,
    pub pattern_name: Option<String>,
    pub style: UIStyleInfo,
    pub colors: ColorPaletteInfo,
    pub typography: TypographyInfo,
    pub key_effects: Option<String>,
    pub avoid_anti_patterns: Vec<String>,
    pub checklist: Vec<ChecklistItem>,
    pub raw_markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct GenerateDesignSystemRequest {
    pub prompt: String,
    pub project_name: Option<String>,
    pub stack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct DesignViolation {
    pub file: String,
    pub line: Option<usize>,
    pub rule: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct AuditDesignSystemResult {
    pub compliant: bool,
    pub total_violations: usize,
    pub violations: Vec<DesignViolation>,
}

#[derive(Debug, thiserror::Error)]
pub enum DesignSystemError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Process failed: {0}")]
    ProcessFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

pub struct DesignSystemService;

const MASTER_DIR: &str = "design-system";
const MASTER_FILE: &str = "MASTER.md";

impl DesignSystemService {
    fn master_path(worktree_path: &Path) -> PathBuf {
        worktree_path.join(MASTER_DIR).join(MASTER_FILE)
    }

    /// Resolve `MASTER.md`, preferring the root
    /// (`{worktree}/design-system/MASTER.md`) and falling back to any
    /// `MASTER.md` inside subdirectories (`design-system/*/MASTER.md`),
    /// since `search.py --persist` may nest the output per project.
    fn resolve_master(worktree_path: &Path) -> Option<PathBuf> {
        let root = Self::master_path(worktree_path);
        if root.is_file() {
            return Some(root);
        }
        let dir = worktree_path.join(MASTER_DIR);
        let entries = std::fs::read_dir(&dir).ok()?;
        // Shallowest match wins: direct children first, then deeper walk.
        let mut deeper = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                continue;
            }
            let candidate = path.join(MASTER_FILE);
            if candidate.is_file() {
                return Some(candidate);
            }
            deeper.push(path);
        }
        let mut stack = deeper;
        while let Some(sub) = stack.pop() {
            let entries = match std::fs::read_dir(&sub) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.file_name().and_then(|n| n.to_str()) == Some(MASTER_FILE) {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Load and parse `{worktree}/design-system/MASTER.md`.
    /// Returns `Ok(None)` when the file does not exist.
    /// Load and parse `{worktree}/design-system/MASTER.md`.
    /// Returns `Ok(None)` when the file does not exist.
    pub fn load_design_system(
        worktree_path: &Path,
    ) -> Result<Option<DesignSystemTokens>, DesignSystemError> {
        let Some(path) = Self::resolve_master(worktree_path) else {
            return Ok(None);
        };
        let content = std::fs::read_to_string(&path)?;
        let dir_name = worktree_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
        Ok(Some(parse_master_markdown(&content, dir_name.as_deref())?))
    }

    /// Write `MASTER.md` (creating the directory) and return parsed tokens.
    pub fn save_design_system(
        worktree_path: &Path,
        content: &str,
    ) -> Result<DesignSystemTokens, DesignSystemError> {
        let dir = worktree_path.join(MASTER_DIR);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(MASTER_FILE), content)?;
        Self::load_design_system(worktree_path)?.ok_or_else(|| {
            DesignSystemError::ParseError("saved MASTER.md could not be reloaded".to_string())
        })
    }

    /// Generate a design system via the universal Python skill script, which
    /// persists `MASTER.md` into the worktree; then load and return it.
    pub fn generate_design_system(
        worktree_path: &Path,
        req: &GenerateDesignSystemRequest,
    ) -> Result<DesignSystemTokens, DesignSystemError> {
        let script = find_skill_script().ok_or_else(|| {
            DesignSystemError::NotFound(
                "ui-ux-pro-max search.py not found under %USERPROFILE%/.agents/skills".to_string(),
            )
        })?;
        // `stack` is informational context for the caller; the script query
        // carries the design intent (`prompt`).
        let _ = req.stack.as_deref();
        let mut cmd = std::process::Command::new("python");
        cmd.arg(&script);
        cmd.arg(&req.prompt);
        cmd.args(["--design-system", "--format", "markdown", "--persist"]);
        cmd.arg("--output-dir");
        cmd.arg(worktree_path.to_string_lossy().as_ref());
        if let Some(project) = req.project_name.as_deref() {
            cmd.args(["-p", project]);
        }
        let status = cmd.status().map_err(DesignSystemError::Io)?;
        if !status.success() {
            return Err(DesignSystemError::ProcessFailed(format!(
                "search.py exited with {status}"
            )));
        }
        // Guarantee the root file as source of truth: if the generator
        // nested the output (design-system/*/MASTER.md), copy it up.
        let root = Self::master_path(worktree_path);
        if !root.is_file() {
            if let Some(nested) = Self::resolve_master(worktree_path) {
                if nested != root {
                    if let Some(parent) = root.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(&nested, &root)?;
                }
            }
        }
        Self::load_design_system(worktree_path)?.ok_or_else(|| {
            DesignSystemError::NotFound(format!(
                "generator succeeded but {} is missing",
                Self::master_path(worktree_path).display()
            ))
        })
    }

    /// Scan `.tsx`/`.jsx`/`.html` files (skipping `node_modules` and `.git`)
    /// for basic design-rule violations.
    pub fn audit_worktree(
        worktree_path: &Path,
    ) -> Result<AuditDesignSystemResult, DesignSystemError> {
        let mut violations = Vec::new();
        let mut stack = vec![worktree_path.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name == "node_modules" || name == ".git" {
                            continue;
                        }
                    }
                    stack.push(path);
                    continue;
                }
                let is_target = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e, "tsx" | "jsx" | "html"))
                    .unwrap_or(false);
                if !is_target {
                    continue;
                }
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let rel = path
                    .strip_prefix(worktree_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| path.to_string_lossy().to_string());
                for (idx, line) in content.lines().enumerate() {
                    let line_no = idx + 1;
                    if let Some(ch) = first_emoji(line) {
                        violations.push(DesignViolation {
                            file: rel.clone(),
                            line: Some(line_no),
                            rule: "no emojis as icons".to_string(),
                            message: format!("emoji '{ch}' used in JSX text; use an icon component"),
                        });
                    }
                    // Heuristic, single-line: elements wiring onClick should
                    // show a pointer cursor.
                    if line.contains("onClick") && !line.contains("cursor-pointer") {
                        violations.push(DesignViolation {
                            file: rel.clone(),
                            line: Some(line_no),
                            rule: "clickable needs cursor-pointer".to_string(),
                            message: "element with onClick lacks the cursor-pointer class"
                                .to_string(),
                        });
                    }
                }
            }
        }
        let total_violations = violations.len();
        Ok(AuditDesignSystemResult {
            compliant: violations.is_empty(),
            total_violations,
            violations,
        })
    }
}

fn find_skill_script() -> Option<PathBuf> {
    let rel = Path::new(".agents")
        .join("skills")
        .join("ui-ux-pro-max")
        .join("scripts")
        .join("search.py");
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(&rel);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// MASTER.md parser (lenient: bullets/bold/plain `Key: value` lines)
// ---------------------------------------------------------------------------

fn parse_master_markdown(
    content: &str,
    dir_fallback: Option<&str>,
) -> Result<DesignSystemTokens, DesignSystemError> {
    let mut sections: HashMap<String, Vec<String>> = HashMap::new();
    let mut preamble = Vec::new();
    let mut current: Option<String> = None;
    let mut title: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(hashes) = trimmed.strip_prefix("### ") {
            current = Some(normalize_section(hashes));
            continue;
        }
        if let Some(hashes) = trimmed.strip_prefix("## ") {
            // Accept `##` too, but a lone `#` is the document title.
            current = Some(normalize_section(hashes));
            continue;
        }
        if title.is_none() {
            if let Some(t) = trimmed.strip_prefix("# ") {
                title = Some(t.trim().to_string());
                continue;
            }
            if !trimmed.is_empty() {
                preamble.push(line.to_string());
            }
            continue;
        }
        match current.as_deref() {
            Some(section) => sections
                .entry(section.to_string())
                .or_default()
                .push(line.to_string()),
            None => preamble.push(line.to_string()),
        }
    }

    let top_kv = parse_kv_lines(&preamble);
    let project_name = match title.as_deref() {
        Some(t) if t == "Design System Master File" => top_kv
            .get("project")
            .cloned()
            .or_else(|| dir_fallback.map(|d| d.to_string()))
            .unwrap_or_else(|| "project".to_string()),
        Some(t) => t
            .strip_prefix("Design System:")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| t.to_string()),
        None => dir_fallback
            .map(|d| d.to_string())
            .unwrap_or_else(|| "project".to_string()),
    };
    let pattern_name = top_kv
        .get("pattern")
        .or_else(|| top_kv.get("pattern name"))
        .cloned()
        .or_else(|| {
            sections
                .get("style")
                .map(|l| parse_kv_lines(l))
                .and_then(|kv| kv.get("pattern").cloned())
        });

    let style_kv = sections
        .get("style")
        .or_else(|| sections.get("style guidelines"))
        .map(|l| parse_kv_lines(l))
        .unwrap_or_default();
    let style = UIStyleInfo {
        name: style_kv
            .get("name")
            .or_else(|| style_kv.get("style"))
            .cloned()
            .ok_or_else(|| DesignSystemError::ParseError("Style.name missing".to_string()))?,
        keywords: style_kv.get("keywords").cloned(),
        best_for: style_kv.get("best for").cloned(),
        performance: style_kv.get("performance").cloned(),
        accessibility: style_kv.get("accessibility").cloned(),
    };

    let colors_lines = sections
        .get("colors")
        .or_else(|| sections.get("color palette"));
    let colors_kv = colors_lines
        .map(|l| parse_kv_lines(l))
        .unwrap_or_default();
    // Extended format: markdown table rows (`| Primary | `#hex` | ...`).
    let table_kv = colors_lines
        .map(|l| parse_color_table(l))
        .unwrap_or_default();
    let color = |key: &str| {
        colors_kv
            .get(key)
            .cloned()
            .or_else(|| table_kv.get(key).cloned())
    };
    let colors = ColorPaletteInfo {
        primary: color("primary")
            .ok_or_else(|| DesignSystemError::ParseError("Colors.primary missing".to_string()))?,
        secondary: color("secondary").unwrap_or_default(),
        cta: color("cta")
            .or_else(|| color("accent"))
            .unwrap_or_default(),
        background: color("background").unwrap_or_default(),
        text: color("text").unwrap_or_default(),
        notes: colors_kv.get("notes").cloned(),
    };

    let typo_kv = sections
        .get("typography")
        .map(|l| parse_kv_lines(l))
        .unwrap_or_default();
    let typography = TypographyInfo {
        heading: typo_kv
            .get("heading")
            .or_else(|| typo_kv.get("heading font"))
            .cloned()
            .ok_or_else(|| {
                DesignSystemError::ParseError("Typography.heading missing".to_string())
            })?,
        body: typo_kv
            .get("body")
            .or_else(|| typo_kv.get("body font"))
            .cloned()
            .unwrap_or_default(),
        mood: typo_kv.get("mood").cloned(),
        best_for: typo_kv.get("best for").cloned(),
        google_fonts: typo_kv.get("google fonts").cloned(),
        css_import: typo_kv.get("css import").cloned(),
    };

    let key_effects = sections
        .get("key effects")
        .map(|lines| lines.join("\n").trim().to_string())
        .filter(|s| !s.is_empty());

    let avoid_anti_patterns = sections
        .get("avoid (anti-patterns)")
        .or_else(|| sections.get("avoid"))
        .map(|lines| bullet_texts(lines))
        .unwrap_or_default();

    let checklist = sections
        .get("pre-delivery checklist")
        .or_else(|| sections.get("checklist"))
        .map(|lines| parse_checklist(lines))
        .unwrap_or_default();

    Ok(DesignSystemTokens {
        project_name,
        pattern_name,
        style,
        colors,
        typography,
        key_effects,
        avoid_anti_patterns,
        checklist,
        raw_markdown: content.to_string(),
    })
}

fn normalize_section(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Parse markdown table rows (`| Name | `#hex` | …`) into lowercase keys.
fn parse_color_table(lines: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in lines {
        let text = line.trim();
        if !(text.starts_with('|') && text.ends_with('|')) {
            continue;
        }
        let cells: Vec<String> = text
            .trim_matches('|')
            .split('|')
            .map(|c| {
                c.trim()
                    .trim_matches('`')
                    .trim()
                    .to_string()
            })
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let key = cells[0].to_lowercase();
        if key.is_empty() || key.chars().all(|c| c == '-' || c == ':' || c == ' ') {
            continue; // separator row
        }
        let value = cells[1].clone();
        if !value.is_empty() {
            map.entry(key).or_insert(value);
        }
    }
    map
}

/// Parse `Key: value` lines, tolerating bullets (`-`, `*`) and bold keys.
fn parse_kv_lines(lines: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in lines {
        let mut text = line.trim();
        if let Some(rest) = text.strip_prefix("- ") {
            text = rest.trim();
        } else if let Some(rest) = text.strip_prefix("* ") {
            text = rest.trim();
        }
        let Some((raw_key, raw_value)) = text.split_once(':') else {
            continue;
        };
        let key = raw_key
            .trim()
            .trim_matches('*')
            .trim()
            .to_lowercase();
        let value = raw_value
            .trim()
            .trim_matches('*')
            .trim()
            .to_string();
        if !key.is_empty() && !value.is_empty() {
            map.entry(key).or_insert(value);
        }
    }
    map
}

fn bullet_texts(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.trim())
        .filter_map(|l| {
            l.strip_prefix("- ")
                .or_else(|| l.strip_prefix("* "))
                .or_else(|| l.strip_prefix("+ "))
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_checklist(lines: &[String]) -> Vec<ChecklistItem> {
    let mut items = Vec::new();
    for line in lines {
        let text = line.trim();
        let body = text
            .strip_prefix("- ")
            .or_else(|| text.strip_prefix("* "))
            .unwrap_or(text);
        if body.is_empty() {
            continue;
        }
        let lower = body.to_lowercase();
        if let Some(rest) = lower.strip_prefix("[x]") {
            items.push(ChecklistItem {
                rule: rest.trim().trim_start_matches(':').trim().to_string(),
                passed: true,
            });
        } else if let Some(rest) = lower.strip_prefix("[ ]") {
            // Keep the original casing of the rule text.
            let rule = body[3..].trim().trim_start_matches(':').trim().to_string();
            let _ = rest;
            items.push(ChecklistItem { rule, passed: false });
        } else if text.starts_with("- ") || text.starts_with("* ") {
            items.push(ChecklistItem {
                rule: body.to_string(),
                passed: false,
            });
        }
    }
    items
}

/// First emoji-ish char on the line, if any.
fn first_emoji(line: &str) -> Option<char> {
    line.chars().find(|c| is_emoji_char(*c))
}

fn is_emoji_char(c: char) -> bool {
    matches!(c,
        '\u{1F600}'..='\u{1F64F}' // emoticons
        | '\u{1F300}'..='\u{1F5FF}' // symbols & pictographs
        | '\u{1F680}'..='\u{1F6FF}' // transport
        | '\u{1F700}'..='\u{1F77F}' // alchemical
        | '\u{1F780}'..='\u{1F7FF}' // geometric ext
        | '\u{1F800}'..='\u{1F8FF}' // arrows supp
        | '\u{1F900}'..='\u{1F9FF}' // supplemental
        | '\u{1FA00}'..='\u{1FA6F}' // chess etc.
        | '\u{1FA70}'..='\u{1FAFF}' // extended-A
        | '\u{2600}'..='\u{26FF}' // misc symbols
        | '\u{2700}'..='\u{27BF}' // dingbats
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL: &str = r#"# Design System: Demo Shop

Pattern: Bento Grid

### Style
- **Name:** Bento
- **Keywords:** modular, cards, dashboard
- **Best for:** SaaS dashboards
- **Performance:** high, CSS only
- **Accessibility:** AA contrast baked in

### Colors
- **Primary:** #111827
- Secondary: #6B7280
- CTA: #4F46E5
- Background: #F9FAFB
- Text: #030712
- Notes: dark CTA on light bg

### Typography
- Heading: Inter Tight
- Body: Inter
- Mood: neutral tech
- Best for: dense UI
- Google Fonts: Inter Tight + Inter
- CSS import: @import url('https://fonts.googleapis.com/css2?family=Inter');

### Key Effects
Soft shadows and 1px borders; no heavy blur.

### Avoid (Anti-patterns)
- Glassmorphism on data tables
- Pure #000 on pure #FFF

### Pre-Delivery Checklist
- [x] Contrast AA verified
- [ ] Focus states visible
"#;

    #[test]
    fn test_parse_canonical_master() {
        let tokens = parse_master_markdown(CANONICAL, None).unwrap();
        assert_eq!(tokens.project_name, "Demo Shop");
        assert_eq!(tokens.pattern_name.as_deref(), Some("Bento Grid"));
        assert_eq!(tokens.style.name, "Bento");
        assert_eq!(tokens.style.keywords.as_deref(), Some("modular, cards, dashboard"));
        assert_eq!(tokens.style.best_for.as_deref(), Some("SaaS dashboards"));
        assert_eq!(tokens.colors.primary, "#111827");
        assert_eq!(tokens.colors.cta, "#4F46E5");
        assert_eq!(tokens.colors.notes.as_deref(), Some("dark CTA on light bg"));
        assert_eq!(tokens.typography.heading, "Inter Tight");
        assert_eq!(
            tokens.typography.google_fonts.as_deref(),
            Some("Inter Tight + Inter")
        );
        assert!(tokens
            .key_effects
            .as_deref()
            .unwrap()
            .contains("Soft shadows"));
        assert_eq!(tokens.avoid_anti_patterns.len(), 2);
        assert_eq!(tokens.checklist.len(), 2);
        assert!(tokens.checklist[0].passed);
        assert!(!tokens.checklist[1].passed);
        assert_eq!(tokens.checklist[1].rule, "Focus states visible");
        assert_eq!(tokens.raw_markdown, CANONICAL);
    }

    const PERSISTED: &str = r#"# Design System Master File

**Project:** Market Ops

### Style Guidelines
- Style: Glassmorphism
- Keywords: frosted, depth

### Color Palette
| Token | Value | Usage |
| --- | --- | --- |
| Primary | `#0F172A` | headers |
| Secondary | `#334155` | body text |
| Accent | `#22D3EE` | CTA buttons |
| Background | `#020617` | page |
| Text | `#F8FAFC` | default |

### Typography
- Heading font: Sora
- Body font: Inter

### Key Effects
Backdrop blur panels.

### Avoid (Anti-patterns)
- Flat gray cards

### Pre-Delivery Checklist
- [x] Contrast AA verified
"#;

    #[test]
    fn test_parse_persisted_master_file() {
        let tokens = parse_master_markdown(PERSISTED, Some("fallback-dir")).unwrap();
        assert_eq!(tokens.project_name, "Market Ops");
        assert_eq!(tokens.style.name, "Glassmorphism");
        assert_eq!(tokens.colors.primary, "#0F172A");
        assert_eq!(tokens.colors.secondary, "#334155");
        assert_eq!(tokens.colors.cta, "#22D3EE");
        assert_eq!(tokens.colors.background, "#020617");
        assert_eq!(tokens.colors.text, "#F8FAFC");
        assert_eq!(tokens.typography.heading, "Sora");
        assert_eq!(tokens.typography.body, "Inter");
        assert_eq!(tokens.avoid_anti_patterns.len(), 1);
        assert!(tokens.checklist[0].passed);

        // Without **Project:**, the directory name wins.
        let no_project = PERSISTED.replace("**Project:** Market Ops\n\n", "");
        let tokens = parse_master_markdown(&no_project, Some("fallback-dir")).unwrap();
        assert_eq!(tokens.project_name, "fallback-dir");
    }

    #[test]
    fn test_tokens_serde_roundtrip() {
        let tokens = parse_master_markdown(CANONICAL, None).unwrap();
        let json = serde_json::to_string(&tokens).unwrap();
        let back: DesignSystemTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(tokens, back);
    }

    #[test]
    fn test_load_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = DesignSystemService::load_design_system(dir.path()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let tokens = DesignSystemService::save_design_system(dir.path(), CANONICAL).unwrap();
        assert_eq!(tokens.project_name, "Demo Shop");
        assert!(dir.path().join("design-system").join("MASTER.md").is_file());
        let reloaded = DesignSystemService::load_design_system(dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(tokens, reloaded);
    }

    #[test]
    fn test_load_prefers_root_over_nested() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("design-system").join("shop");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("MASTER.md"), CANONICAL).unwrap();
        // Nested copy resolves when root is missing.
        let tokens = DesignSystemService::load_design_system(dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(tokens.project_name, "Demo Shop");
        // Root wins once it exists.
        DesignSystemService::save_design_system(dir.path(), CANONICAL).unwrap();
        let tokens = DesignSystemService::load_design_system(dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(tokens.project_name, "Demo Shop");
    }

    #[test]
    fn test_audit_finds_violations() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        std::fs::write(
            dir.path().join("node_modules").join("bad.tsx"),
            "<div onClick={f}>🚀 rompelo</div>",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("App.tsx"),
            "<button onClick={save}>Guardar 🎉</button>",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("ok.tsx"),
            "<button onClick={save} className=\"cursor-pointer\">Guardar</button>",
        )
        .unwrap();

        let result = DesignSystemService::audit_worktree(dir.path()).unwrap();
        assert!(!result.compliant);
        // App.tsx: 1 emoji + 1 missing cursor-pointer = 2 violations.
        // node_modules/bad.tsx must be ignored.
        assert_eq!(result.total_violations, 2);
        assert!(result
            .violations
            .iter()
            .any(|v| v.rule == "no emojis as icons"));
        assert!(result
            .violations
            .iter()
            .any(|v| v.rule == "clickable needs cursor-pointer"));
        assert!(result.violations.iter().all(|v| v.file == "App.tsx"));
    }
}
