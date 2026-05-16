# Clashtui Development Conventions

## Branch Management

| Branch  | Purpose               |
| ------- | --------------------- |
| `main`  | Main branch, stable   |
| `dev`   | Development branch    |

## Branch Naming Convention

```
feature/[issue/discussion-id]-xxx-xxx
improvement/[issue/discussion-id]-xxx-xxx
bugfix/[issue/discussion-id]-xxx-xxx
```

- `feature` — New features
- `improvement` — Improvements and optimizations
- `bugfix` — Bug fixes
- `[issue/discussion-id]` — Linked Issue or Discussion ID, e.g. `#42`
- `xxx-xxx` — Short description, separated by hyphens

Examples:

```
feature/#42-add-proxy-provider
improvement/#15-refactor-config-parser
bugfix/#8-fix-dns-leak
```

## Commit Conventions

### Before Committing

```sh
cargo fmt
```

Ensure code passes formatting checks before committing.

### Commit Message Format

Follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

#### Type

| type       | Description                |
| ---------- | -------------------------- |
| `feat`     | New feature                |
| `fix`      | Bug fix                    |
| `improve`  | Improvement                |
| `refactor` | Code refactoring           |
| `perf`     | Performance optimization   |
| `docs`     | Documentation              |
| `chore`    | Build/tooling/dependencies |
| `test`     | Testing                    |
| `ci`       | CI/CD                      |

Examples:

```
feat: add proxy provider support

fix(config): correct DNS rule parsing

improve(ui): optimize connection list rendering
```

- For breaking changes, add `BREAKING CHANGE:` in the footer.
- Reference related issues in the body: `Closes #42` or `Refs #15`.

## CHANGELOG

A `CHANGELOG.md` is maintained at the project root, following the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

### When to Update

Update the CHANGELOG **when releasing a new version**. Before tagging a release, summarize changes since the last release into `CHANGELOG.md`, grouped as follows:

| Group        | Description              |
| ------------ | ------------------------ |
| `Added`      | New features             |
| `Changed`    | Changes in functionality |
| `Deprecated` | Soon-to-be removed       |
| `Removed`    | Removed features         |
| `Fixed`      | Bug fixes                |
| `Security`   | Security fixes           |

Do not modify CHANGELOG in daily commits — only update it at release time. Since commit messages follow Conventional Commits, tools like [git-cliff](https://github.com/orhun/git-cliff) can auto-generate the changelog.
