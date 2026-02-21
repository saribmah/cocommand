//! Terminal operation implementations.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::error::{CoreError, CoreResult};

const MAX_OUTPUT_CHARS: usize = 30_000;
const MAX_GLOB_RESULTS: usize = 100;
const MAX_GREP_MATCHES: usize = 100;
const MAX_LINE_CHARS: usize = 2_000;
const MAX_LS_ENTRIES: usize = 100;

const DEFAULT_IGNORE: &[&str] = &[
    "node_modules",
    "__pycache__",
    ".git",
    "dist",
    "build",
    "target",
    "vendor",
    ".idea",
    ".vscode",
    ".cache",
    "venv",
    ".venv",
    "coverage",
    "tmp",
    "temp",
    "logs",
];

fn truncate_output(s: &str, max: usize) -> (String, bool) {
    if s.len() <= max {
        (s.to_string(), false)
    } else {
        let truncated = &s[..s.floor_char_boundary(max)];
        (truncated.to_string(), true)
    }
}

/// Execute a shell command via `/bin/sh -c`.
pub async fn bash_exec(
    command: &str,
    timeout_ms: u64,
    workdir: &Path,
) -> CoreResult<serde_json::Value> {
    let start = Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    let child = tokio::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .current_dir(workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::Internal(format!("failed to spawn shell: {e}")))?;

    let result = tokio::time::timeout(timeout, child.wait_with_output()).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(Ok(output)) => {
            let (stdout, _) =
                truncate_output(&String::from_utf8_lossy(&output.stdout), MAX_OUTPUT_CHARS);
            let (stderr, _) =
                truncate_output(&String::from_utf8_lossy(&output.stderr), MAX_OUTPUT_CHARS);
            let exit_code = output.status.code().unwrap_or(-1);

            Ok(serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "exitCode": exit_code,
                "timedOut": false,
                "command": command,
                "workdir": workdir.to_string_lossy(),
                "durationMs": duration_ms,
            }))
        }
        Ok(Err(e)) => Err(CoreError::Internal(format!("command failed: {e}"))),
        Err(_) => {
            // Timed out — the child process is dropped which sends SIGKILL
            Ok(serde_json::json!({
                "stdout": "",
                "stderr": format!("command timed out after {timeout_ms}ms"),
                "exitCode": -1,
                "timedOut": true,
                "command": command,
                "workdir": workdir.to_string_lossy(),
                "durationMs": duration_ms,
            }))
        }
    }
}

/// Find files matching a glob pattern, sorted by mtime descending.
pub fn glob_files(pattern: &str, path: &Path) -> CoreResult<serde_json::Value> {
    let full_pattern = path.join(pattern);
    let full_pattern_str = full_pattern.to_string_lossy();

    let entries = glob::glob(&full_pattern_str)
        .map_err(|e| CoreError::InvalidInput(format!("invalid glob pattern: {e}")))?;

    let mut files_with_mtime: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    for entry in entries {
        if files_with_mtime.len() >= MAX_GLOB_RESULTS {
            break;
        }
        if let Ok(path) = entry {
            let mtime = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            files_with_mtime.push((path, mtime));
        }
    }

    // Sort by mtime descending (newest first)
    files_with_mtime.sort_by(|a, b| b.1.cmp(&a.1));

    let truncated = files_with_mtime.len() >= MAX_GLOB_RESULTS;
    let files: Vec<String> = files_with_mtime
        .iter()
        .map(|(p, _)| p.to_string_lossy().to_string())
        .collect();
    let count = files.len();

    Ok(serde_json::json!({
        "files": files,
        "count": count,
        "truncated": truncated,
        "pattern": pattern,
        "path": path.to_string_lossy(),
    }))
}

/// Search file contents via regex using grep or ripgrep.
pub fn grep_files(
    pattern: &str,
    path: &Path,
    include: Option<&str>,
) -> CoreResult<serde_json::Value> {
    let has_rg = std::process::Command::new("which")
        .arg("rg")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let output = if has_rg {
        let mut cmd = std::process::Command::new("rg");
        cmd.args(["-nH", "--no-messages", "--hidden", "--color=never"]);
        if let Some(glob) = include {
            cmd.args(["--glob", glob]);
        }
        cmd.arg(pattern).arg(path);
        cmd.output()
    } else {
        let mut cmd = std::process::Command::new("grep");
        cmd.args(["-rnH", "--color=never"]);
        if let Some(glob) = include {
            cmd.args(["--include", glob]);
        }
        cmd.arg(pattern).arg(path);
        cmd.output()
    };

    let output = output.map_err(|e| CoreError::Internal(format!("grep failed: {e}")))?;
    let raw = String::from_utf8_lossy(&output.stdout);

    // Parse lines and group by file
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut total_matches = 0;

    for line in raw.lines() {
        if total_matches >= MAX_GREP_MATCHES {
            break;
        }

        // Format: filename:lineno:content
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() == 3 {
            let filename = parts[0].to_string();
            let lineno = parts[1];
            let mut content = parts[2].to_string();
            if content.len() > MAX_LINE_CHARS {
                content.truncate(content.floor_char_boundary(MAX_LINE_CHARS));
                content.push_str("...");
            }
            grouped
                .entry(filename)
                .or_default()
                .push(format!("  {lineno}: {content}"));
            total_matches += 1;
        }
    }

    let truncated = total_matches >= MAX_GREP_MATCHES;

    // Build indented tree output
    let mut tree = String::new();
    for (file, lines) in &grouped {
        tree.push_str(file);
        tree.push('\n');
        for line in lines {
            tree.push_str(line);
            tree.push('\n');
        }
    }

    Ok(serde_json::json!({
        "output": tree.trim_end(),
        "matches": total_matches,
        "truncated": truncated,
        "pattern": pattern,
        "path": path.to_string_lossy(),
    }))
}

/// List directory tree with ignore patterns, BFS walk.
pub fn list_dir(path: &Path, user_ignore: &[String]) -> CoreResult<serde_json::Value> {
    if !path.is_dir() {
        return Err(CoreError::InvalidInput(format!(
            "path is not a directory: {}",
            path.display()
        )));
    }

    let mut ignore_set: std::collections::HashSet<String> =
        DEFAULT_IGNORE.iter().map(|s| s.to_string()).collect();
    for pattern in user_ignore {
        ignore_set.insert(pattern.clone());
    }

    let mut output = String::new();
    let mut count = 0;
    let mut truncated = false;

    // BFS walk
    let mut queue: std::collections::VecDeque<(PathBuf, usize)> = std::collections::VecDeque::new();
    queue.push_back((path.to_path_buf(), 0));

    while let Some((current, depth)) = queue.pop_front() {
        if count >= MAX_LS_ENTRIES {
            truncated = true;
            break;
        }

        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        let mut children: Vec<std::fs::DirEntry> = entries.filter_map(|e| e.ok()).collect();
        children.sort_by_key(|e| e.file_name());

        for entry in children {
            if count >= MAX_LS_ENTRIES {
                truncated = true;
                break;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            // Skip ignored directories
            if is_dir && ignore_set.contains(&name) {
                continue;
            }

            let indent = "  ".repeat(depth);
            if is_dir {
                output.push_str(&format!("{indent}{name}/\n"));
                count += 1;
                queue.push_back((entry.path(), depth + 1));
            } else {
                output.push_str(&format!("{indent}{name}\n"));
                count += 1;
            }
        }
    }

    Ok(serde_json::json!({
        "output": output.trim_end(),
        "count": count,
        "truncated": truncated,
        "path": path.to_string_lossy(),
    }))
}
