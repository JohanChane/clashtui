## Why

The current keybinding system uses a flat `HashMap<KeyEvent, Action>` mapping — every key maps to exactly one action. There is no support for multi-key sequences (chords like `gg`, `Ctrl-w c`, etc.), which limits the expressiveness and ergonomics of keyboard-driven interaction. Additionally, when a user starts a multi-key sequence, there is no visual feedback showing what keys are available next (a "which" / shortcut panel). This change introduces a Chord-based keybinding system with a built-in which panel, inspired by yazi's design.

## What Changes

- **BREAKING**: Replace `mod_agent!` macro's `HashMap<KeyEvent, Action>` with `Vec<Chord>` where `Chord` is a key sequence (`Vec<Key>`) paired with a runnable command
- Introduce a custom `Key` type (code + shift/ctrl/alt/super flags) replacing direct crossterm `KeyEvent` usage in keybinding, removing the `Press`/`Repeat`/`Release` distinction from matching logic
- Add a `Router` component that sits before Tabs and matches incoming keys against the active layer's Chord list, handling both single-key instant execution and multi-key sequence accumulation
- Add a `Which` global state (independent of PopUp) that renders a shortcut panel showing available chord candidates when the user starts a multi-key prefix
- Key event dispatch order becomes: `Which (if active)` → `PopUp` → `App global keys` → `Tab layer`
- TuiWidget's `handle_key_event` is replaced with a command/action dispatch — Tabs no longer receive raw key events, they declare their Chords upfront

## Capabilities

### New Capabilities
- `chord-keybinding`: Chord-based keybinding system — define multi-key sequences, match against incoming Key events, route to commands
- `which-panel`: Shortcut panel that renders available chord candidates during multi-key input, with progressive filtering and auto-execution

### Modified Capabilities
- *(none — no existing specs to modify)*

## Impact

- `src/tui/tab/mod.rs` — `mod_agent!` macro replaced, `TuiWidget` / `TuiTab` traits modified
- `src/tui/app.rs` — new routing logic, `Which` state added, `handle_key_event` dispatch order changed
- `src/tui/widget/popmsg` — no changes (Which is independent of PopUp)
- `src/tui/agent.rs` — restructured to support Chord definitions per layer
- `src/tui/theme.rs` — new theme section for Which panel rendering
- All existing Tab implementations (`StatusTab`, `FileTab`, sub-contents) — key handling refactored from `handle_key_event(&KeyEvent)` to chord-based command dispatch
