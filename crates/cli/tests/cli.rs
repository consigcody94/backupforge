//! Drives the real `backupforge` binary.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn run(args: &[&str], password: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_backupforge"));
    cmd.args(args)
        .env_remove("BACKUPFORGE_PASSWORD")
        .stdin(std::process::Stdio::null());
    if let Some(pw) = password {
        cmd.env("BACKUPFORGE_PASSWORD", pw);
    }
    cmd.output().unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn every_subcommand_parses_its_own_help() {
    // Catches clap definition bugs (duplicate flags etc.) that panic at startup.
    for sub in ["init", "backup", "restore", "list", "verify", "stats"] {
        let out = run(&[sub, "--help"], None);
        assert!(
            out.status.success(),
            "{sub} --help failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn encrypted_backup_list_verify_restore() {
    let tmp = TempDir::new().unwrap();
    let (src, repo, out) = (
        tmp.path().join("src"),
        tmp.path().join("repo"),
        tmp.path().join("out"),
    );
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("a.txt"), "hello cli").unwrap();
    fs::write(src.join("sub/b.txt"), "second file").unwrap();
    let pw = Some("cli test password");

    let r = run(&["backup", "-s", s(&src), "-d", s(&repo), "--encrypt"], pw);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));

    let list = run(&["list", "-d", s(&repo)], pw);
    assert!(list.status.success());
    assert!(String::from_utf8_lossy(&list.stdout).contains("2 files"));

    assert!(run(&["verify", "-d", s(&repo)], pw).status.success());

    let restore = run(&["restore", "-d", s(&repo), "-t", s(&out)], pw);
    assert!(
        restore.status.success(),
        "{}",
        String::from_utf8_lossy(&restore.stderr)
    );
    assert_eq!(fs::read_to_string(out.join("a.txt")).unwrap(), "hello cli");
    assert_eq!(
        fs::read_to_string(out.join("sub/b.txt")).unwrap(),
        "second file"
    );
}

#[test]
fn wrong_password_fails_with_nonzero_exit() {
    let tmp = TempDir::new().unwrap();
    let (src, repo) = (tmp.path().join("src"), tmp.path().join("repo"));
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), "x").unwrap();
    assert!(run(
        &["backup", "-s", s(&src), "-d", s(&repo), "-e"],
        Some("right")
    )
    .status
    .success());

    assert!(!run(&["list", "-d", s(&repo)], Some("wrong"))
        .status
        .success());
}

#[test]
fn read_commands_do_not_create_a_repository() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("nope");
    for args in [vec!["list"], vec!["verify"], vec!["stats"]] {
        let mut a = args.clone();
        a.extend(["-d", s(&repo)]);
        assert!(!run(&a, None).status.success(), "{args:?}");
    }
    let out = tmp.path().join("out");
    assert!(!run(&["restore", "-d", s(&repo), "-t", s(&out)], None)
        .status
        .success());
    assert!(!repo.exists());
}

#[test]
fn init_refuses_to_overwrite_an_existing_repository() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    assert!(run(&["init", "-d", s(&repo)], None).status.success());
    assert!(!run(&["init", "-d", s(&repo)], None).status.success());
}

#[test]
fn excluded_paths_are_left_out() {
    let tmp = TempDir::new().unwrap();
    let (src, repo, out) = (
        tmp.path().join("src"),
        tmp.path().join("repo"),
        tmp.path().join("out"),
    );
    fs::create_dir_all(src.join("node_modules")).unwrap();
    fs::write(src.join("keep.txt"), "keep").unwrap();
    fs::write(src.join("node_modules/skip.txt"), "skip").unwrap();

    assert!(run(
        &[
            "backup",
            "-s",
            s(&src),
            "-d",
            s(&repo),
            "-x",
            "node_modules"
        ],
        None
    )
    .status
    .success());
    assert!(run(&["restore", "-d", s(&repo), "-t", s(&out)], None)
        .status
        .success());
    assert!(out.join("keep.txt").exists());
    assert!(!out.join("node_modules").exists());
}
