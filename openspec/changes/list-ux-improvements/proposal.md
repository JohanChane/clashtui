## Why

The profile and template tabs lack several UX conveniences present in other tabs (connections, proxies). Navigation is inconsistent — arrow keys work but `j`/`k` don't, `ge` is used for jump-to-end instead of the expected `G`, template has no search, delete has no confirmation, and cursor positions aren't preserved when switching between tabs. These gaps make list navigation frustrating and inconsistent.

## What Changes

- Add file import action trigger in profile tab (`I` key), prompting for source file path and profile name via PopUp
- Add `dd` (double-tap `d`) chord for file deletion in both profile and template tabs, with a confirmation popup before deleting
- Change `ge` chord to single `G` key in profile and template tabs. Add `gg`, `G` support to template tab
- Add `j`/`k` vim-style cursor movement bindings alongside existing arrow keys in profile and template tabs
- Add `/` search/filter PopUp to template tab, reusing the existing `filter` field and rendering logic
- Implement cursor position memory across tab switches: cursor snaps to first item on initial entry, then remembers and validates its row on subsequent switches. Applies to profile, template, proxies, and connections tabs

## Capabilities

### New Capabilities

- `delete-confirmation`: Confirmation dialog before deleting profile YAML files or template files via `dd` chord
- `template-search`: Search/filter in template list via `/` key, reusing existing filter rendering
- `cursor-position-memory`: Cursor position persistence across tab switches with validity clamping

### Modified Capabilities

- `shortcut-bindings`: Profile and template bindings now include `G`, `j`, `k`, `dd`; `ge` chord removed in favor of `G`
- `file-path-import`: Profile tab now exposes `I` key binding to trigger local file import

## Impact

- **Files modified**: `src/tui/tab/files/profile.rs`, `src/tui/tab/files/template.rs`, `src/tui/tab/connections.rs`, `src/tui/tab/proxies/content.rs`
- **New files**: none expected
- **Breaking keybindings**: `ge` chord removed (replaced by `G`); `d` single-key delete removed from profile (replaced by `dd` chord); user keymap.yaml overrides using `ge` or single `d` will need updating
- **No API or database changes**
