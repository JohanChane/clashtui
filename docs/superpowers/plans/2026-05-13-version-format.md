# Version Format Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Simplify build version to `{CARGO_PKG_VERSION}-{short_hash}[-dirty]`, removing dependency on git tags.

**Architecture:** Single-file change in `build.rs`: replace `git describe --always --tags` with `git rev-parse --short HEAD`, remove branch_name and build_type logic, drop `v` prefix.

**Tech Stack:** Rust build script, git CLI

---

### Task 1: Update build.rs

**Files:**
- Modify: `build.rs`

- [ ] **Step 1: Replace `get_version()` implementation**

Replace the entire `get_version()` function in `build.rs` with:

```rust
fn get_version() -> String {
    let cargo_pkg_version = env::var("CARGO_PKG_VERSION").unwrap();

    let git_short_hash = match Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(v) => String::from_utf8(v.stdout)
            .expect("failed to read stdout")
            .trim_end()
            .to_string(),
        Err(err) => {
            eprintln!("`git rev-parse` err: {}", err);
            "unknown".to_string()
        }
    };

    let dirty = match Command::new("git")
        .args(["status", "--short"])
        .output()
    {
        Ok(v) => {
            if v.stdout.is_empty() {
                String::new()
            } else {
                "-dirty".to_string()
            }
        }
        Err(e) => {
            eprintln!("`git status --short` err: {e}");
            String::new()
        }
    };

    format!("{cargo_pkg_version}-{git_short_hash}{dirty}")
}
```

- [ ] **Step 2: Build and verify version**

Run: `cargo build`
Check the output binary version:
Run: `cargo run -- --version`
Expected: prints version like `0.1.0-abc1234` or `0.1.0-abc1234-dirty`

- [ ] **Step 3: Commit**

```bash
git add build.rs
git commit -m "simplify version format to {semver}-{short_hash}[-dirty]"
```
