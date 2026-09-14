use std::path::{Path, PathBuf};

/// Directory name for storing attachments in worktrees
pub const VIBE_ATTACHMENTS_DIR: &str = ".vibe-attachments";

/// Directories that should always be skipped regardless of gitignore.
/// .git is not in .gitignore but should never be watched.
pub const ALWAYS_SKIP_DIRS: &[&str] = &[".git", "node_modules"];

/// The Windows extended-length path prefix: backslash, backslash, question mark, backslash.
/// Kept as a named constant because miscounting the backslashes is silent and self-consistent
/// — a wrong literal makes both the code and its test agree on the wrong answer.
const VERBATIM_PREFIX: &str = r"\\?\";
/// What follows [`VERBATIM_PREFIX`] when the path is a UNC share.
const VERBATIM_UNC_INFIX: &str = r"UNC\";

/// Convert absolute paths to relative paths based on worktree path
/// This is a robust implementation that handles symlinks and edge cases
pub fn make_path_relative(path: &str, worktree_path: &str) -> String {
    tracing::trace!("Making path relative: {} -> {}", path, worktree_path);

    let path_obj = normalize_macos_private_alias(Path::new(&path));
    let worktree_path_obj = normalize_macos_private_alias(Path::new(worktree_path));

    // If path is already relative, return as is
    if path_obj.is_relative() {
        return path.to_string();
    }

    if let Ok(relative_path) = path_obj.strip_prefix(&worktree_path_obj) {
        let result = relative_path.to_string_lossy().to_string();
        tracing::trace!("Successfully made relative: '{}' -> '{}'", path, result);
        if result.is_empty() {
            return ".".to_string();
        }
        return result;
    }

    if !path_obj.exists() || !worktree_path_obj.exists() {
        return path.to_string();
    }

    // canonicalize may fail if paths don't exist
    let canonical_path = std::fs::canonicalize(&path_obj);
    let canonical_worktree = std::fs::canonicalize(&worktree_path_obj);

    match (canonical_path, canonical_worktree) {
        (Ok(canon_path), Ok(canon_worktree)) => {
            tracing::debug!(
                "Trying canonical path resolution: '{}' -> '{}', '{}' -> '{}'",
                path,
                canon_path.display(),
                worktree_path,
                canon_worktree.display()
            );

            match canon_path.strip_prefix(&canon_worktree) {
                Ok(relative_path) => {
                    let result = relative_path.to_string_lossy().to_string();
                    tracing::debug!(
                        "Successfully made relative with canonical paths: '{}' -> '{}'",
                        path,
                        result
                    );
                    if result.is_empty() {
                        return ".".to_string();
                    }
                    result
                }
                Err(e) => {
                    tracing::debug!(
                        "Failed to make canonical path relative: '{}' relative to '{}', error: {}, returning original",
                        canon_path.display(),
                        canon_worktree.display(),
                        e
                    );
                    path.to_string()
                }
            }
        }
        _ => {
            tracing::debug!(
                "Could not canonicalize paths (paths may not exist): '{}', '{}', returning original",
                path,
                worktree_path
            );
            path.to_string()
        }
    }
}

/// Normalize macOS prefix /private/var/ and /private/tmp/ to their public aliases without resolving paths.
/// This allows prefix normalization to work when the full paths don't exist.
pub fn normalize_macos_private_alias<P: AsRef<Path>>(p: P) -> PathBuf {
    let p = p.as_ref();
    if cfg!(target_os = "macos")
        && let Some(s) = p.to_str()
    {
        if s == "/private/var" {
            return PathBuf::from("/var");
        }
        if let Some(rest) = s.strip_prefix("/private/var/") {
            return PathBuf::from(format!("/var/{rest}"));
        }
        if s == "/private/tmp" {
            return PathBuf::from("/tmp");
        }
        if let Some(rest) = s.strip_prefix("/private/tmp/") {
            return PathBuf::from(format!("/tmp/{rest}"));
        }
    }
    p.to_path_buf()
}

/// Strip the Windows extended-length ("verbatim") `\\?\` prefix that `canonicalize` adds.
///
/// `std::fs::canonicalize` returns `\\?\C:\Users\...` on Windows, while paths stored in
/// the database — container refs, worktree paths — are plain `C:\Users\...`. Comparing
/// the two as strings never matches, so any lookup keyed on a canonicalized path silently
/// fails. This is the Windows counterpart of [`normalize_macos_private_alias`], which
/// exists for the same class of bug on macOS.
pub fn strip_windows_verbatim_prefix<P: AsRef<Path>>(p: P) -> PathBuf {
    let p = p.as_ref();
    if cfg!(windows)
        && let Some(s) = p.to_str()
        && let Some(rest) = s.strip_prefix(VERBATIM_PREFIX)
    {
        // Verbatim UNC paths (`\\?\UNC\server\share`) map back to `\\server\share`.
        if let Some(unc) = rest.strip_prefix(VERBATIM_UNC_INFIX) {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        return PathBuf::from(rest);
    }
    p.to_path_buf()
}

pub fn get_vibe_kanban_temp_dir() -> std::path::PathBuf {
    let dir_name = if cfg!(debug_assertions) {
        "vibe-kanban-dev"
    } else {
        "vibe-kanban"
    };

    if cfg!(target_os = "macos") {
        // macOS already uses /var/folders/... which is persistent storage
        std::env::temp_dir().join(dir_name)
    } else if cfg!(target_os = "linux") {
        // Linux: use /var/tmp instead of /tmp to avoid RAM usage
        std::path::PathBuf::from("/var/tmp").join(dir_name)
    } else {
        // Windows and other platforms: use temp dir with vibe-kanban subdirectory
        std::env::temp_dir().join(dir_name)
    }
}

/// Expand leading ~ to user's home directory.
pub fn expand_tilde(path_str: &str) -> std::path::PathBuf {
    shellexpand::tilde(path_str).as_ref().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Absolute (per host) forward-slash path under the temp dir. A drive-less
    /// literal like `/tmp/test-worktree` parses as *relative* on Windows, which
    /// skips the stripping logic these cases exercise.
    fn temp_path(name: &str) -> String {
        let prefix = std::env::temp_dir()
            .display()
            .to_string()
            .replace('\\', "/");
        format!("{prefix}/{name}")
    }

    #[test]
    fn test_make_path_relative() {
        let worktree = temp_path("test-worktree");
        // Test with relative path (should remain unchanged)
        assert_eq!(make_path_relative("src/main.rs", &worktree), "src/main.rs");

        // Test with absolute path (should become relative if possible)
        let absolute_path = format!("{worktree}/src/main.rs");
        let result = make_path_relative(&absolute_path, &worktree);
        assert_eq!(result, "src/main.rs");

        // Test with path outside worktree (should return original)
        let outside = format!("{}/file.js", temp_path("other"));
        assert_eq!(make_path_relative(&outside, &worktree), outside);
    }

    /// Pin the prefix byte by byte.
    ///
    /// Escaped byte literals are unambiguous in a way a raw string is not: if the constant
    /// ever loses a backslash, this fails, whereas a behavioural test written with the same
    /// wrong literal would agree with the bug and pass.
    #[test]
    fn test_verbatim_prefix_is_exactly_four_bytes() {
        assert_eq!(VERBATIM_PREFIX.as_bytes(), &[b'\\', b'\\', b'?', b'\\']);
        assert_eq!(VERBATIM_UNC_INFIX.as_bytes(), &[b'U', b'N', b'C', b'\\']);
    }

    #[cfg(windows)]
    #[test]
    fn test_strip_windows_verbatim_prefix() {
        // What canonicalize() returns, against what the database stores.
        let canonicalized = format!("{VERBATIM_PREFIX}C:\\Users\\x\\worktrees\\vk-test");
        assert_eq!(
            strip_windows_verbatim_prefix(&canonicalized),
            PathBuf::from("C:\\Users\\x\\worktrees\\vk-test")
        );
        // Verbatim UNC paths go back to their ordinary form.
        let unc = format!("{VERBATIM_PREFIX}{VERBATIM_UNC_INFIX}server\\share\\dir");
        assert_eq!(
            strip_windows_verbatim_prefix(&unc),
            PathBuf::from("\\\\server\\share\\dir")
        );
        // Without the prefix, untouched.
        assert_eq!(
            strip_windows_verbatim_prefix("C:\\Users\\x"),
            PathBuf::from("C:\\Users\\x")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_make_path_relative_macos_private_alias() {
        // Simulate a worktree under /var with a path reported under /private/var
        let worktree = "/var/folders/zz/abc123/T/vibe-kanban-dev/worktrees/vk-test";
        let path_under_private = format!(
            "/private/var{}/hello-world.txt",
            worktree.strip_prefix("/var").unwrap()
        );
        assert_eq!(
            make_path_relative(&path_under_private, worktree),
            "hello-world.txt"
        );

        // Also handle the inverse: worktree under /private and path under /var
        let worktree_private = format!("/private{worktree}");
        let path_under_var = format!("{worktree}/hello-world.txt");
        assert_eq!(
            make_path_relative(&path_under_var, &worktree_private),
            "hello-world.txt"
        );
    }
}
