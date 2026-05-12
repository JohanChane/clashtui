# Version Format Redesign

## Motivation

GitHub Actions 默认 checkout 不拉取 tag，导致 `git describe --tags` 无法工作。同时当前格式包含过多冗余信息（分支名、debug/release 标记、前缀 `v`），简化后更通用且更可靠。

## Design

### Format

```
{CARGO_PKG_VERSION}-{short_commit_hash}[-dirty]
```

| 部分 | 来源 | 说明 |
|------|------|------|
| version | `CARGO_PKG_VERSION` (0.1.0) | semver，Cargo.toml 唯一权威来源 |
| hash | `git rev-parse --short HEAD` | 短 commit hash (7-8 chars)，不依赖 tag |
| dirty | `git status --short` 输出非空 | 有未提交修改时追加 |

### Examples

- `0.1.0-abc1234` — 干净构建
- `0.1.0-abc1234-dirty` — 有未提交修改

### What's removed

- `v` 前缀 — 冗余，`CARGO_PKG_VERSION` 不含
- `{branch_name}` — 调试罕见需要分支名，commit hash 足够定位
- `{build_type}` (debug/release) — 调试可通过 binary 大小或 `file` 命令判断

### What stays

- `-dirty` — 关键区分本地未提交修改 vs 干净构建（cargo、ripgrep、fd 等都用此约定）

### Build-time fallback

若 git 不可用（无 `.git` 目录、未安装 git），hash 退化为 `unknown`，dirty 标记退化为空字符串：
- `0.1.0-unknown`

### Impact

- `build.rs`: 替换 `git describe --always --tags` 为 `git rev-parse --short HEAD`
- `build.rs`: 移除 branch_name、build_type 采集
- `src/cli/utils.rs`: 无需更改（仍通过 `env!("CLASHTUI_VERSION")` 读取）
- `--version` CLI 输出格式变化，不涉及代码语义
