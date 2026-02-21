//! Terminal extension tests.

use std::fs;
use std::io::Write;

use super::extension::TerminalExtension;
use super::ops;

#[test]
fn terminal_extension_has_expected_tools() {
    let ext = TerminalExtension::new();
    let tools = crate::extension::Extension::tools(&ext);
    assert_eq!(tools.len(), 4);
    assert_eq!(tools[0].id, "bash");
    assert_eq!(tools[1].id, "glob");
    assert_eq!(tools[2].id, "grep");
    assert_eq!(tools[3].id, "ls");
}

#[tokio::test]
async fn bash_exec_echo() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = ops::bash_exec("echo hello", 120_000, dir.path())
        .await
        .expect("bash_exec");
    assert_eq!(result["stdout"].as_str().unwrap().trim(), "hello");
    assert_eq!(result["exitCode"].as_i64().unwrap(), 0);
    assert!(!result["timedOut"].as_bool().unwrap());
}

#[tokio::test]
async fn bash_exec_timeout() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = ops::bash_exec("sleep 10", 100, dir.path())
        .await
        .expect("bash_exec timeout");
    assert!(result["timedOut"].as_bool().unwrap());
    assert_eq!(result["exitCode"].as_i64().unwrap(), -1);
}

#[tokio::test]
async fn bash_exec_exit_code() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = ops::bash_exec("exit 42", 120_000, dir.path())
        .await
        .expect("bash_exec exit code");
    assert_eq!(result["exitCode"].as_i64().unwrap(), 42);
    assert!(!result["timedOut"].as_bool().unwrap());
}

#[test]
fn glob_finds_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("a.txt"), "hello").expect("write a.txt");
    fs::write(dir.path().join("b.txt"), "world").expect("write b.txt");
    fs::write(dir.path().join("c.rs"), "fn main() {}").expect("write c.rs");

    let result = ops::glob_files("*.txt", dir.path()).expect("glob_files");
    let files = result["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(result["count"].as_u64().unwrap(), 2);
    assert!(!result["truncated"].as_bool().unwrap());
}

#[test]
fn grep_finds_pattern() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("test.txt");
    let mut f = fs::File::create(&file_path).expect("create file");
    writeln!(f, "line one").expect("write");
    writeln!(f, "line two with pattern_match here").expect("write");
    writeln!(f, "line three").expect("write");

    let result = ops::grep_files("pattern_match", dir.path(), None).expect("grep_files");
    assert!(result["matches"].as_u64().unwrap() >= 1);
    let output = result["output"].as_str().unwrap();
    assert!(output.contains("pattern_match"));
}

#[test]
fn list_dir_basic() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir(dir.path().join("subdir")).expect("mkdir subdir");
    fs::write(dir.path().join("file.txt"), "hello").expect("write");
    fs::write(dir.path().join("subdir/nested.txt"), "nested").expect("write nested");

    let result = ops::list_dir(dir.path(), &[]).expect("list_dir");
    let output = result["output"].as_str().unwrap();
    assert!(output.contains("file.txt"));
    assert!(output.contains("subdir/"));
    assert!(result["count"].as_u64().unwrap() >= 2);
}

#[test]
fn list_dir_ignores_patterns() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir(dir.path().join(".git")).expect("mkdir .git");
    fs::create_dir(dir.path().join("src")).expect("mkdir src");
    fs::write(dir.path().join(".git/config"), "").expect("write");
    fs::write(dir.path().join("src/main.rs"), "").expect("write");

    let result = ops::list_dir(dir.path(), &[]).expect("list_dir");
    let output = result["output"].as_str().unwrap();
    assert!(!output.contains(".git"), ".git should be ignored");
    assert!(output.contains("src/"));
}
