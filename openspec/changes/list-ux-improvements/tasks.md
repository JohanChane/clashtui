## 1. Profile tab keymap changes

- [x] 1.1 Add `j` and `k` single-key bindings to `mod_agent!` in `src/tui/tab/files/profile.rs`
- [x] 1.2 Add `G` single-key binding (GoEnd) and remove `g e` chord (GoEnd) from profile `mod_agent!`
- [x] 1.3 Replace single `d` key binding with `d d` chord (Action::Delete) in profile `mod_agent!`
- [x] 1.4 Update profile `Action::act()` to pass GoTop/GoEnd/Delete through correctly (GoTop/GoEnd already handled in handle_key_event, but verify Delete path works with choreged key dispatch)
- [x] 1.5 Add `j`/`k` resolution in profile `TryFrom<&Key>` manual fallback (or verify agent handles it)

## 2. Profile delete with confirmation

- [x] 2.1 Modify `actions::delete()` in `src/tui/tab/files/profile.rs` to show a Confirm popup before deleting
- [x] 2.2 Cancel path: if user cancels, return `do_nothing()` without deleting
- [x] 2.3 Confirm path: proceed with `db::remove()` and `sync!` as before

## 3. Template tab keymap changes

- [x] 3.1 Add `j` and `k` single-key bindings to template `mod_agent!` in `src/tui/tab/files/template.rs`
- [x] 3.2 Add `G` single-key binding (GoEnd), remove `g e` chord (GoEnd) from template `mod_agent!`
- [x] 3.3 Replace single `d` key binding with `d d` chord (Action::Delete) in template `mod_agent!`
- [x] 3.4 Add `/` single-key binding (Action::Search) to template `mod_agent!`
- [x] 3.5 Add `Search` variant to template `Action` enum
- [x] 3.6 Add `j`/`k`/other fallback resolution in template `TryFrom<&Key>` (or verify agent handles it)

## 4. Template delete implementation

- [x] 4.1 Implement `actions::delete()` in `src/tui/tab/files/template.rs` (currently `todo!()`): show Confirm popup, remove template file from `templates/` directory, refresh template list
- [x] 4.2 Handle file-not-found gracefully (template already deleted from filesystem)
- [x] 4.3 Ensure delete path works with chord dispatch (verify `dd` chord triggers the delete async handler)

## 5. Template search implementation

- [x] 5.1 Implement `async fn search()` in template actions (copy pattern from profile's `search()`): show Input PopUp with "Filter" title
- [x] 5.2 Wire `Action::Search` to the `search()` function in `Action::act()`
- [x] 5.3 Add `Action::Search` to the match arm in template's `handle_key_event` (or verify it routes through the generic `_` arm)

## 6. Cursor position memory

- [x] 6.1 Profile: clamp `ListState` selected index in render (ensures initial `Some(0)`, valid after content changes)
- [x] 6.2 Profile: cursor selects first item on initial render when state is None and items non-empty
- [x] 6.3 Template: clamp `ListState` selected index in render
- [x] 6.4 Template: cursor selects first item on initial render when state is None and items non-empty
- [x] 6.5 Connections: in `refresh_display_rows()`, clamp `self.row` to `< display_rows.len()`, default to `Some(0)` if `None` and list is non-empty
- [x] 6.6 Proxies: clamp `ListState` selected index in render

## 7. Build & verify

- [x] 7.1 Run `cargo check` to verify all code compiles
- [x] 7.2 Run `cargo test` to ensure existing tests pass
- [x] 7.3 Run `cargo build --release` to verify release build succeeds
