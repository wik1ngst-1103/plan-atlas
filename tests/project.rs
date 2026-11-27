//! Project naming from a working directory.

use std::fs;

use plan_atlas::project::project_root_name;

#[test]
fn normal_repo_uses_repo_dir_name() {
    let t = tempfile::tempdir().unwrap();
    let repo = t.path().join("My Repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(repo.join("src/deep")).unwrap();
    assert_eq!(
        project_root_name(&repo.join("src/deep")).as_deref(),
        Some("my-repo")
    );
}

#[test]
fn worktree_uses_main_repo_name() {
    let t = tempfile::tempdir().unwrap();
    let main = t.path().join("atlas");
    let gitdir = main.join(".git/worktrees/feature-x");
    fs::create_dir_all(&gitdir).unwrap();
    fs::write(gitdir.join("commondir"), "../..\n").unwrap();
    let wt = t.path().join("worktrees/feature-x");
    fs::create_dir_all(wt.join("web")).unwrap();
    fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();
    assert_eq!(project_root_name(&wt.join("web")).as_deref(), Some("atlas"));
}

#[test]
fn worktree_with_relative_gitdir() {
    let t = tempfile::tempdir().unwrap();
    let main = t.path().join("atlas");
    let gitdir = main.join(".git/worktrees/wt");
    fs::create_dir_all(&gitdir).unwrap();
    fs::write(gitdir.join("commondir"), "../..").unwrap();
    let wt = main.join(".claude/worktrees/wt");
    fs::create_dir_all(&wt).unwrap();
    fs::write(wt.join(".git"), "gitdir: ../../../.git/worktrees/wt\n").unwrap();
    assert_eq!(project_root_name(&wt).as_deref(), Some("atlas"));
}

#[test]
fn no_repo_uses_cwd_name() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("scratch_pad");
    fs::create_dir_all(&dir).unwrap();
    assert_eq!(project_root_name(&dir).as_deref(), Some("scratch-pad"));
}
