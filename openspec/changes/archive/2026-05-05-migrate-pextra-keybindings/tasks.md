## 1. Config: Expose config directory path

- [x] 1.1 Add `pub fn config_dir_path() -> PathBuf` to `src/config.rs`, returning `DATA_DIR.get().unwrap().clone()`

## 2. Functions: Add `open_dir()` command

- [x] 2.1 Add `pub fn open_dir(path: &str) -> Result<()>` to `src/functions/command.rs`, mirroring `edit()` but using `open_dir_cmd`

## 3. TUI: Add global chord layer for Ctrl+g c / Ctrl+g m

- [x] 3.1 Add `global_chord: ChordHandler` field to `App` struct in `src/tui/app.rs`
- [x] 3.2 Define `GLOBAL_CHORD_SHORTCUTS` static with `[Ctrl+g, c]` → "Open app config dir" and `[Ctrl+g, m]` → "Open clash config dir" entries (using const-fn to construct `Key` values with `ctrl: true`)
- [x] 3.3 Insert global chord layer in `handle_key_event` between PopUp (layer 0) and Help (layer 1): use `self.global_chord.handle()` with dispatch calling `open_dir()` for `c`/`m` second keys

## 4. Template tab: Add e key to edit template files

- [x] 4.1 Add `use crate::functions::command::edit;` import to `src/tui/tab/files/template.rs`
- [x] 4.2 Add `Edit` variant to the `Action` enum in template.rs
- [x] 4.3 Add `([KeyCode::Char('e')], Key::Action(Action::Edit), "")` binding to `mod_agent!`
- [x] 4.4 Add `Self::Edit => _edit(name).await` arm to `Action::act()`
- [x] 4.5 Add `async fn _edit(name: String) -> CB` using `edit(config::template_path().join(&name).to_str().unwrap())`

## 5. Help panel: Document new global chords

- [x] 5.1 Add `Ctrl+g c` and `Ctrl+g m` entries to `global_shortcuts` and `global_labels` in `src/tui/widget/help.rs`

## 6. Verification

- [x] 6.1 Run `cargo check` to ensure the code compiles
- [x] 6.2 Run `cargo test` to ensure no regressions
