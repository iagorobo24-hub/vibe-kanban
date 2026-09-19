use std::{
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DetectionConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct DetectedScripts {
    pub setup_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub dev_server_script: Option<String>,
    pub copy_files: Option<String>,
    pub parallel_setup_script: bool,
    pub confidence: DetectionConfidence,
    pub stack_description: String,
    pub detection_notes: Vec<String>,
}

pub struct ScriptDetectionService;

struct NodeInfo {
    package_manager: &'static str,
    install_cmd: String,
    dev_cmd: Option<String>,
    cleanup_cmd: Option<String>,
    is_typescript: bool,
    is_monorepo: bool,
}

struct RustInfo {
    is_workspace: bool,
}

struct PythonInfo {
    setup_cmd: String,
    cleanup_cmd: Option<String>,
    tool_name: &'static str,
}

struct GoInfo {
    module_name: Option<String>,
}

impl ScriptDetectionService {
    /// Detects scripts and project configuration based on files present in `repo_path`.
    pub fn detect_scripts(repo_path: &Path) -> DetectedScripts {
        let mut notes = Vec::new();

        if !repo_path.exists() {
            return DetectedScripts {
                setup_script: None,
                cleanup_script: None,
                dev_server_script: None,
                copy_files: None,
                parallel_setup_script: false,
                confidence: DetectionConfidence::Low,
                stack_description: "Directorio no encontrado".to_string(),
                detection_notes: vec![format!(
                    "La ruta {} no existe",
                    repo_path.to_string_lossy()
                )],
            };
        }

        // 1. Detect copy_files (.env.example / .env.sample)
        let copy_files = Self::detect_copy_files(repo_path, &mut notes);

        // 1b. Detect env example for inline copy (copy_files has no backend
        // consumer, so the setup script must do the copy itself).
        let env_copy = Self::detect_env_copy(repo_path, &mut notes);

        // 2. Detect language stacks
        let node_info = Self::detect_node(repo_path, &mut notes);
        let rust_info = Self::detect_rust(repo_path, &mut notes);
        let python_info = Self::detect_python(repo_path, &mut notes);
        let go_info = Self::detect_go(repo_path, &mut notes);

        // 3. Synthesize results
        let mut setup_lines = Vec::new();
        let mut cleanup_lines = Vec::new();
        let mut dev_server = None;
        let mut stacks = Vec::new();
        let mut confidence = DetectionConfidence::Low;

        // Rust
        if let Some(rust) = rust_info {
            confidence = DetectionConfidence::High;
            if rust.is_workspace {
                stacks.push("Rust (Workspace)");
            } else {
                stacks.push("Rust (Cargo)");
            }
            setup_lines.push("cargo build".to_string());
            cleanup_lines.push("cargo fmt".to_string());
        }

        // Node.js
        if let Some(node) = node_info {
            confidence = DetectionConfidence::High;
            let mut node_label = format!("Node.js ({})", node.package_manager);
            if node.is_typescript {
                node_label.push_str(" + TypeScript");
            }
            if node.is_monorepo {
                node_label.push_str(" (Monorepo)");
            }
            stacks.push(Box::leak(node_label.into_boxed_str()));

            setup_lines.push(node.install_cmd);
            if let Some(env) = &env_copy {
                setup_lines.push(env.clone());
            }
            if let Some(cleanup) = node.cleanup_cmd {
                cleanup_lines.push(cleanup);
            }
            if dev_server.is_none() && let Some(dev) = node.dev_cmd {
                dev_server = Some(dev);
            }
        }

        // Python
        if let Some(py) = python_info {
            if confidence == DetectionConfidence::Low {
                confidence = DetectionConfidence::High;
            }
            stacks.push(Box::leak(format!("Python ({})", py.tool_name).into_boxed_str()));
            setup_lines.push(py.setup_cmd);
            if let Some(cleanup) = py.cleanup_cmd {
                cleanup_lines.push(cleanup);
            }
        }

        // Go
        if let Some(go) = go_info {
            if confidence == DetectionConfidence::Low {
                confidence = DetectionConfidence::High;
            }
            if let Some(mod_name) = go.module_name {
                stacks.push(Box::leak(format!("Go ({})", mod_name).into_boxed_str()));
            } else {
                stacks.push("Go");
            }
            setup_lines.push("go mod download".to_string());
            cleanup_lines.push("gofmt -w .".to_string());
        }

        // Fallback: Makefile if no primary stack found
        if stacks.is_empty() && let Some(make_setup) = Self::detect_makefile(repo_path, &mut notes) {
            confidence = DetectionConfidence::Medium;
            stacks.push("Makefile");
            setup_lines.push(make_setup);
        }

        let setup_script = if setup_lines.is_empty() {
            None
        } else {
            // Single cmd-safe line: setup runs as `cmd /C`, where `&&`
            // chains preserve real exit codes. Steps carry their own
            // `if not exist` guards so retries never fight a previous run.
            Some(setup_lines.join(" && "))
        };

        let cleanup_script = if cleanup_lines.is_empty() {
            None
        } else {
            Some(cleanup_lines.join("\n"))
        };

        let stack_description = if stacks.is_empty() {
            "No identificado".to_string()
        } else {
            stacks.join(" + ")
        };

        if confidence == DetectionConfidence::Low {
            notes.push("No se detectaron archivos de configuración de stack estándar (package.json, Cargo.toml, etc.)".to_string());
        }

        DetectedScripts {
            setup_script,
            cleanup_script,
            dev_server_script: dev_server,
            copy_files,
            parallel_setup_script: false,
            confidence,
            stack_description,
            detection_notes: notes,
        }
    }

    fn detect_copy_files(repo_path: &Path, notes: &mut Vec<String>) -> Option<String> {
        let has_env_example = repo_path.join(".env.example").is_file();
        let has_env_sample = repo_path.join(".env.sample").is_file();
        let has_env = repo_path.join(".env").is_file();

        if has_env_example || has_env_sample || has_env {
            notes.push("Detectado archivo de entorno (.env/.env.example). Configurado copy_files para .env".to_string());
            Some(".env".to_string())
        } else {
            None
        }
    }

    /// Inline env copy for the setup script itself (`copy_files` is stored
    /// for the UI but has no backend consumer).
    fn detect_env_copy(repo_path: &Path, notes: &mut Vec<String>) -> Option<String> {
        let example = if repo_path.join(".env.example").is_file() {
            ".env.example"
        } else if repo_path.join(".env.sample").is_file() {
            ".env.sample"
        } else {
            return None;
        };
        notes.push(format!("Detectado {example}; copia inline con guard en setup"));
        Some(format!("if not exist .env copy {example} .env"))
    }

    fn detect_node(repo_path: &Path, notes: &mut Vec<String>) -> Option<NodeInfo> {
        let pkg_path = repo_path.join("package.json");
        if !pkg_path.is_file() {
            return None;
        }

        let package_manager = if repo_path.join("pnpm-lock.yaml").exists() {
            notes.push("Detectado pnpm vía pnpm-lock.yaml".to_string());
            "pnpm"
        } else if repo_path.join("yarn.lock").exists() {
            notes.push("Detectado yarn vía yarn.lock".to_string());
            "yarn"
        } else if repo_path.join("bun.lockb").exists() || repo_path.join("bun.lock").exists() {
            notes.push("Detectado bun vía bun.lock".to_string());
            "bun"
        } else {
            notes.push("Detectado npm por defecto para Node.js".to_string());
            "npm"
        };

        // Frozen installs for reproducible fresh worktrees; guarded so
        // retries never reinstall over a previous run (lock contention).
        let has_lockfile = repo_path.join("pnpm-lock.yaml").exists()
            || repo_path.join("yarn.lock").exists()
            || repo_path.join("bun.lockb").exists()
            || repo_path.join("bun.lock").exists()
            || repo_path.join("package-lock.json").exists();
        let raw_install = match package_manager {
            "pnpm" if has_lockfile => "pnpm install --frozen-lockfile",
            "pnpm" => "pnpm install",
            "yarn" if has_lockfile => "yarn install --frozen-lockfile",
            "yarn" => "yarn install",
            "bun" if has_lockfile => "bun install --frozen-lockfile",
            "bun" => "bun install",
            _ if has_lockfile => "npm ci",
            _ => "npm install",
        };
        if has_lockfile {
            notes.push("Detectado lockfile; instalación frozen reproducible".to_string());
        }
        let install_cmd = format!("if not exist node_modules {raw_install}");

        let is_typescript = repo_path.join("tsconfig.json").is_file();
        let is_monorepo = repo_path.join("turbo.json").is_file()
            || repo_path.join("pnpm-workspace.yaml").is_file()
            || repo_path.join("lerna.json").is_file();

        if is_monorepo {
            notes.push("Detectado monorepo Node.js".to_string());
        }

        let mut dev_cmd = None;
        let mut cleanup_cmd = None;

        if let Ok(content) = fs::read_to_string(&pkg_path)
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
        {
            if let Some(scripts) = val.get("scripts").and_then(|s| s.as_object()) {
                // Dev server detection
                if scripts.contains_key("dev") {
                    dev_cmd = Some(match package_manager {
                        "npm" => "npm run dev".to_string(),
                        pm => format!("{} dev", pm),
                    });
                } else if scripts.contains_key("start") {
                    dev_cmd = Some(match package_manager {
                        "npm" => "npm start".to_string(),
                        pm => format!("{} start", pm),
                    });
                }

                // Cleanup detection (lint / format)
                if scripts.contains_key("lint:fix") {
                    cleanup_cmd = Some(match package_manager {
                        "npm" => "npm run lint:fix".to_string(),
                        pm => format!("{} run lint:fix", pm),
                    });
                } else if scripts.contains_key("format") {
                    cleanup_cmd = Some(match package_manager {
                        "npm" => "npm run format".to_string(),
                        pm => format!("{} run format", pm),
                    });
                } else if scripts.contains_key("lint") {
                    cleanup_cmd = Some(match package_manager {
                        "npm" => "npm run lint -- --fix".to_string(),
                        pm => format!("{} run lint --fix", pm),
                    });
                } else if scripts.contains_key("check") {
                    cleanup_cmd = Some(match package_manager {
                        "npm" => "npm run check".to_string(),
                        pm => format!("{} run check", pm),
                    });
                }
            }

            // Fallback for prettier if no script found
            if cleanup_cmd.is_none()
                && (repo_path.join(".prettierrc").exists()
                    || repo_path.join(".prettierrc.json").exists()
                    || repo_path.join("prettier.config.js").exists()
                    || repo_path.join("prettier.config.mjs").exists())
            {
                notes.push("Detectada configuración de Prettier".to_string());
                cleanup_cmd = Some("npx prettier --write .".to_string());
            }
        }

        Some(NodeInfo {
            package_manager,
            install_cmd,
            dev_cmd,
            cleanup_cmd,
            is_typescript,
            is_monorepo,
        })
    }

    fn detect_rust(repo_path: &Path, notes: &mut Vec<String>) -> Option<RustInfo> {
        let cargo_path = repo_path.join("Cargo.toml");
        if !cargo_path.is_file() {
            return None;
        }

        let is_workspace = if let Ok(content) = fs::read_to_string(&cargo_path) {
            content.contains("[workspace]")
        } else {
            false
        };

        if is_workspace {
            notes.push("Detectado workspace de Rust con Cargo.toml".to_string());
        } else {
            notes.push("Detectado proyecto Rust con Cargo.toml".to_string());
        }

        Some(RustInfo { is_workspace })
    }

    fn detect_python(repo_path: &Path, notes: &mut Vec<String>) -> Option<PythonInfo> {
        let has_pyproject = repo_path.join("pyproject.toml").is_file();
        let has_reqs = repo_path.join("requirements.txt").is_file();
        let has_pipfile = repo_path.join("Pipfile").is_file();
        let has_poetry_lock = repo_path.join("poetry.lock").is_file();

        if !has_pyproject && !has_reqs && !has_pipfile {
            return None;
        }

        let (setup_cmd, tool_name) = if has_poetry_lock {
            notes.push("Detectado Poetry vía poetry.lock".to_string());
            ("poetry install".to_string(), "poetry")
        } else if has_pipfile {
            notes.push("Detectado Pipfile".to_string());
            ("pipenv install".to_string(), "pipenv")
        } else if has_reqs {
            notes.push("Detectado requirements.txt".to_string());
            ("pip install -r requirements.txt".to_string(), "pip")
        } else {
            notes.push("Detectado pyproject.toml".to_string());
            ("pip install -e .".to_string(), "pip")
        };

        // Detect cleanup formatter
        let cleanup_cmd = if repo_path.join(".ruff.toml").exists()
            || repo_path.join("ruff.toml").exists()
        {
            notes.push("Detectada configuración de Ruff para Python".to_string());
            Some("ruff format . && ruff check --fix .".to_string())
        } else if repo_path.join(".black").exists() || repo_path.join("pyproject.toml").exists() {
            // Check if black is mentioned in pyproject
            if let Ok(content) = fs::read_to_string(repo_path.join("pyproject.toml"))
                && (content.contains("[tool.ruff]") || content.contains("[tool.black]"))
            {
                if content.contains("[tool.ruff]") {
                    Some("ruff format . && ruff check --fix .".to_string())
                } else {
                    Some("black .".to_string())
                }
            } else {
                None
            }
        } else {
            None
        };

        Some(PythonInfo {
            setup_cmd,
            cleanup_cmd,
            tool_name,
        })
    }

    fn detect_go(repo_path: &Path, notes: &mut Vec<String>) -> Option<GoInfo> {
        let go_mod_path = repo_path.join("go.mod");
        if !go_mod_path.is_file() {
            return None;
        }

        let module_name = if let Ok(content) = fs::read_to_string(&go_mod_path) {
            content
                .lines()
                .find(|l| l.starts_with("module "))
                .map(|l| l.trim_start_matches("module ").trim().to_string())
        } else {
            None
        };

        notes.push("Detectado proyecto Go vía go.mod".to_string());

        Some(GoInfo { module_name })
    }

    fn detect_makefile(repo_path: &Path, notes: &mut Vec<String>) -> Option<String> {
        let makefile_path = repo_path.join("Makefile");
        if !makefile_path.is_file() {
            return None;
        }

        if let Ok(content) = fs::read_to_string(&makefile_path) {
            if content.contains("setup:") {
                notes.push("Detectado target 'setup' en Makefile".to_string());
                return Some("make setup".to_string());
            } else if content.contains("install:") {
                notes.push("Detectado target 'install' en Makefile".to_string());
                return Some("make install".to_string());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_empty_dir() {
        let dir = tempdir().unwrap();
        let result = ScriptDetectionService::detect_scripts(dir.path());
        assert_eq!(result.confidence, DetectionConfidence::Low);
        assert_eq!(result.setup_script, None);
        assert_eq!(result.cleanup_script, None);
    }

    #[test]
    fn test_node_pnpm_detection() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "test", "scripts": {"dev": "vite", "lint": "eslint ."}}"#,
        )
        .unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();

        let result = ScriptDetectionService::detect_scripts(dir.path());
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert_eq!(
            result.setup_script,
            Some("if not exist node_modules pnpm install --frozen-lockfile".to_string())
        );
        assert_eq!(result.dev_server_script, Some("pnpm dev".to_string()));
        assert_eq!(
            result.cleanup_script,
            Some("pnpm run lint --fix".to_string())
        );
        assert!(result.stack_description.contains("pnpm"));
        assert!(result.stack_description.contains("TypeScript"));
    }

    #[test]
    fn test_rust_cargo_detection() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"
edition = "2024"
"#,
        )
        .unwrap();

        let result = ScriptDetectionService::detect_scripts(dir.path());
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert_eq!(result.setup_script, Some("cargo build".to_string()));
        assert_eq!(result.cleanup_script, Some("cargo fmt".to_string()));
        assert!(result.stack_description.contains("Rust"));
    }

    #[test]
    fn test_hybrid_rust_and_node() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "test", "scripts": {"lint:fix": "eslint --fix ."}}"#,
        )
        .unwrap();

        let result = ScriptDetectionService::detect_scripts(dir.path());
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert!(result.stack_description.contains("Rust"));
        assert!(result.stack_description.contains("Node.js"));
        assert!(result.setup_script.unwrap().contains("cargo build"));
    }

    #[test]
    fn test_python_requirements_detection() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("requirements.txt"), "fastapi==0.100.0").unwrap();
        fs::write(dir.path().join(".ruff.toml"), "").unwrap();

        let result = ScriptDetectionService::detect_scripts(dir.path());
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert_eq!(
            result.setup_script,
            Some("pip install -r requirements.txt".to_string())
        );
        assert_eq!(
            result.cleanup_script,
            Some("ruff format . && ruff check --fix .".to_string())
        );
    }

    #[test]
    fn test_npm_ci_with_lockfile_and_env_copy() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), r#"{"name": "test"}"#).unwrap();
        fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
        fs::write(dir.path().join(".env.example"), "PORT=3000").unwrap();

        let result = ScriptDetectionService::detect_scripts(dir.path());
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert_eq!(
            result.setup_script,
            Some(
                "if not exist node_modules npm ci && if not exist .env copy .env.example .env"
                    .to_string()
            )
        );
        assert_eq!(result.copy_files, Some(".env".to_string()));
    }

    #[test]
    fn test_setup_script_is_single_cmd_line() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "test", "scripts": {"lint:fix": "eslint --fix ."}}"#,
        )
        .unwrap();

        let result = ScriptDetectionService::detect_scripts(dir.path());
        let setup = result.setup_script.unwrap();
        assert!(!setup.contains('\n'), "setup must be a single cmd line");
        assert!(setup.contains("cargo build"));
        assert!(setup.contains("if not exist node_modules"));
    }

    #[test]
    fn test_copy_files_detection() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".env.example"), "PORT=3000").unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let result = ScriptDetectionService::detect_scripts(dir.path());
        assert_eq!(result.copy_files, Some(".env".to_string()));
    }
}
