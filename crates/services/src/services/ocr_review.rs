use std::{path::Path, process::Command};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};
use ts_rs::TS;

#[derive(Debug, Error)]
pub enum OcrReviewError {
    #[error("IO error running OCR CLI: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse OCR output: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Worktree directory not found: {0}")]
    WorktreeNotFound(String),
    #[error("OCR process exited with error: {0}")]
    ProcessFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OcrReviewableFile {
    pub path: String,
    pub status: String,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OcrRuleGroup {
    pub group_id: usize,
    pub pattern: String,
    pub files: Vec<String>,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OcrFinding {
    pub file: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub severity: String,
    pub rule: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OcrReviewResult {
    pub mode: String, // "delegation" | "direct"
    pub total_files: usize,
    pub total_insertions: usize,
    pub total_deletions: usize,
    pub reviewable_files: Vec<OcrReviewableFile>,
    pub rule_groups: Vec<OcrRuleGroup>,
    pub findings: Vec<OcrFinding>,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
struct RawDelegatePreview {
    #[serde(default)]
    total_files: usize,
    #[serde(default)]
    total_insertions: usize,
    #[serde(default)]
    total_deletions: usize,
    #[serde(default)]
    reviewable_files: Vec<RawReviewableFile>,
}

#[derive(Debug, Deserialize)]
struct RawReviewableFile {
    path: String,
    status: String,
    #[serde(default)]
    insertions: usize,
    #[serde(default)]
    deletions: usize,
}

#[derive(Debug, Deserialize)]
struct RawDelegateRules {
    #[serde(default)]
    groups: Vec<RawRuleGroup>,
}

#[derive(Debug, Deserialize)]
struct RawRuleGroup {
    group_id: usize,
    #[serde(default)]
    pattern: String,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    rule: String,
}

pub struct OcrReviewService;

impl OcrReviewService {
    pub fn run_preview_and_rules(worktree_path: &Path) -> Result<OcrReviewResult, OcrReviewError> {
        if !worktree_path.exists() {
            return Err(OcrReviewError::WorktreeNotFound(
                worktree_path.to_string_lossy().to_string(),
            ));
        }

        // Handle nested repositories within the workspace container directory
        let target_repo_path = if worktree_path.join(".git").exists() {
            worktree_path.to_path_buf()
        } else if let Ok(entries) = std::fs::read_dir(worktree_path) {
            let mut found = None;
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.join(".git").exists() {
                    found = Some(p);
                    break;
                }
            }
            found.unwrap_or_else(|| worktree_path.to_path_buf())
        } else {
            worktree_path.to_path_buf()
        };

        info!(
            "Running Alibaba OCR delegate preview in worktree repo: {:?}",
            target_repo_path
        );

        let preview_output = if cfg!(windows) {
            Command::new("cmd")
                .args([
                    "/c",
                    "npx",
                    "-y",
                    "@alibaba-group/open-code-review@1.12.2",
                    "delegate",
                    "preview",
                    "--format",
                    "json",
                ])
                .current_dir(&target_repo_path)
                .output()?
        } else {
            Command::new("npx")
                .args([
                    "-y",
                    "@alibaba-group/open-code-review@1.12.2",
                    "delegate",
                    "preview",
                    "--format",
                    "json",
                ])
                .current_dir(&target_repo_path)
                .output()?
        };

        if !preview_output.status.success() {
            let stderr = String::from_utf8_lossy(&preview_output.stderr);
            warn!("OCR delegate preview failed: {}", stderr);
            return Err(OcrReviewError::ProcessFailed(stderr.to_string()));
        }

        let preview_stdout = String::from_utf8_lossy(&preview_output.stdout);
        let preview: RawDelegatePreview = serde_json::from_str(&preview_stdout)?;

        let reviewable_files: Vec<OcrReviewableFile> = preview
            .reviewable_files
            .into_iter()
            .map(|f| OcrReviewableFile {
                path: f.path,
                status: f.status,
                insertions: f.insertions,
                deletions: f.deletions,
            })
            .collect();

        if reviewable_files.is_empty() {
            return Ok(OcrReviewResult {
                mode: "delegation".to_string(),
                total_files: 0,
                total_insertions: 0,
                total_deletions: 0,
                reviewable_files: vec![],
                rule_groups: vec![],
                findings: vec![],
                summary: "No se detectaron cambios pendientes de revisión en este espacio de trabajo.".to_string(),
            });
        }

        // Run delegate rule for the reviewable files (capped at first 10 files to keep command short)
        let file_args: Vec<&str> = reviewable_files
            .iter()
            .take(10)
            .map(|f| f.path.as_str())
            .collect();

        let mut rule_cmd_args = vec![
            "delegate",
            "rule",
            "--format",
            "json",
        ];
        rule_cmd_args.extend_from_slice(&file_args);

        let rules_output = if cfg!(windows) {
            let mut full_args = vec!["/c", "npx", "-y", "@alibaba-group/open-code-review@1.12.2"];
            full_args.extend(rule_cmd_args);
            Command::new("cmd")
                .args(full_args)
                .current_dir(&target_repo_path)
                .output()?
        } else {
            let mut full_args = vec!["-y", "@alibaba-group/open-code-review@1.12.2"];
            full_args.extend(rule_cmd_args);
            Command::new("npx")
                .args(full_args)
                .current_dir(&target_repo_path)
                .output()?
        };

        let rule_groups = if rules_output.status.success() {
            let rules_stdout = String::from_utf8_lossy(&rules_output.stdout);
            let parsed_rules: Result<RawDelegateRules, _> = serde_json::from_str(&rules_stdout);
            parsed_rules
                .map(|r| {
                    r.groups
                        .into_iter()
                        .map(|g| OcrRuleGroup {
                            group_id: g.group_id,
                            pattern: g.pattern,
                            files: g.files,
                            rule: g.rule,
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };

        let summary = format!(
            "Se detectaron {} archivo(s) modificados (+{} / -{} líneas). Reglas especializadas cargadas para {} grupo(s).",
            preview.total_files,
            preview.total_insertions,
            preview.total_deletions,
            rule_groups.len()
        );

        Ok(OcrReviewResult {
            mode: "delegation".to_string(),
            total_files: preview.total_files,
            total_insertions: preview.total_insertions,
            total_deletions: preview.total_deletions,
            reviewable_files,
            rule_groups,
            findings: vec![],
            summary,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delegate_preview_json() {
        let json = r#"{
          "schema_version": "1",
          "mode": "workspace",
          "repository": "C:/fake/repo",
          "total_files": 2,
          "reviewable_count": 2,
          "excluded_count": 0,
          "total_insertions": 15,
          "total_deletions": 3,
          "reviewable_files": [
            {
              "path": "src/main.rs",
              "status": "modified",
              "insertions": 10,
              "deletions": 2
            },
            {
              "path": "src/lib.rs",
              "status": "added",
              "insertions": 5,
              "deletions": 1
            }
          ],
          "excluded_files": []
        }"#;

        let parsed: RawDelegatePreview = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.total_files, 2);
        assert_eq!(parsed.total_insertions, 15);
        assert_eq!(parsed.total_deletions, 3);
        assert_eq!(parsed.reviewable_files.len(), 2);
        assert_eq!(parsed.reviewable_files[0].path, "src/main.rs");
    }

    #[test]
    fn test_parse_delegate_rules_json() {
        let json = r#"{
          "schema_version": "1",
          "groups": [
            {
              "group_id": 1,
              "source": "system",
              "pattern": "**/*.rs",
              "files": ["src/main.rs"],
              "rule": "Rule description"
            }
          ]
        }"#;

        let parsed: RawDelegateRules = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.groups.len(), 1);
        assert_eq!(parsed.groups[0].group_id, 1);
        assert_eq!(parsed.groups[0].pattern, "**/*.rs");
        assert_eq!(parsed.groups[0].files, vec!["src/main.rs"]);
    }
}
