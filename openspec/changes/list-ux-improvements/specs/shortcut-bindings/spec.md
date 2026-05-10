## MODIFIED Requirements

### Requirement: mod_agent! generates unified shortcuts

The `mod_agent!` macro SHALL generate a `shortcuts()` function that returns ALL key bindings (single-key and multi-key) as a slice of `(key_sequence, action, description)` tuples where `key_sequence` is `Vec<Key>`. The existing `agent()` and `agent_init()` SHALL remain for keymap.yaml compatibility but SHALL use `Key` as the HashMap key type instead of `KeyEvent`.

#### Scenario: Single-key binding in shortcuts
- **WHEN** `mod_agent!` is invoked with `(KeyCode::Char('i'), Key::Action(Action::Add), "Add profile")`
- **THEN** `shortcuts()` returns an entry `(vec![Key { code: Char('i'), ... }], Key::Action(Add), "Add profile")`

#### Scenario: Multi-key chord in shortcuts
- **WHEN** `mod_agent!` is invoked with `(KeyCode::Char('g'), KeyCode::Char('g'), Key::Action(Action::GoTop), "Go to top")`
- **THEN** `shortcuts()` returns an entry `(vec![Key { code: Char('g'), ... }, Key { code: Char('g'), ... }], Key::Action(GoTop), "Go to top")`

#### Scenario: Modifier key in shortcuts
- **WHEN** `mod_agent!` is invoked with `(<C-c>, Key::Close, "Close tab")`
- **THEN** `shortcuts()` returns an entry `(vec![Key { code: Char('c'), ctrl: true }], Key::Close, "Close tab")`

#### Scenario: Mixed single-key and chord entries
- **WHEN** a tab defines both single-key bindings and chord bindings in `mod_agent!`
- **THEN** `shortcuts()` returns one consolidated slice containing both types as `Vec<(Vec<Key>, &str)>`

#### Scenario: Profile tab includes dd chord
- **WHEN** Profile tab's `mod_agent!` defines `([KeyCode::Char('d'), KeyCode::Char('d')], Key::Action(Action::Delete), "Delete profile")`
- **THEN** `shortcuts()` SHALL include the `dd` chord as a 2-key sequence

#### Scenario: Profile tab includes j/k single keys
- **WHEN** Profile tab's `mod_agent!` defines `([KeyCode::Char('j')], Key::MoveDown, "Move down")`
- **THEN** `shortcuts()` SHALL include `j` as a single-key binding for MoveDown

### Requirement: Profile and Template tab key bindings

The Profile tab SHALL expose the following default key bindings: `j` (MoveDown), `k` (MoveUp), `d d` (Delete with confirmation), `G` (GoEnd), `g g` (GoTop), `I` (ImportFile), `/` (Search), `i` (Add), `e` (Edit), `p` (Preview), `u` (Update), `a u` (UpdateAll), `t` (Test), `N` (ToggleNoPp), arrow keys (MoveUp/MoveDown), and Enter (Select).

The Template tab SHALL expose the following default key bindings: `j` (MoveDown), `k` (MoveUp), `d d` (Delete with confirmation), `G` (GoEnd), `g g` (GoTop), `/` (Search), `e` (Edit), `p` (Preview), `Enter` (Generate), and arrow keys (MoveUp/MoveDown).

#### Scenario: Profile tab j/k movement
- **WHEN** user presses `j` in Profile tab
- **THEN** cursor SHALL move down one item
- **WHEN** user presses `k` in Profile tab
- **THEN** cursor SHALL move up one item

#### Scenario: Profile tab G jump-to-end
- **WHEN** user presses `G` (Shift-g) in Profile tab
- **THEN** cursor SHALL move to the last item in the list

#### Scenario: Template tab G and gg navigation
- **WHEN** user presses `G` in Template tab
- **THEN** cursor SHALL move to the last template
- **WHEN** user presses `g` then `g` in Template tab
- **THEN** cursor SHALL move to the first template

#### Scenario: Template tab dd deletion chord
- **WHEN** user presses `d` then `d` in Template tab
- **THEN** the delete-with-confirmation flow SHALL be triggered

#### Scenario: Template tab / search
- **WHEN** user presses `/` in Template tab
- **THEN** a filter PopUp SHALL appear

#### Scenario: ge chord no longer available
- **WHEN** user presses `g` then `e` in Profile or Template tab
- **THEN** no GoEnd action SHALL be triggered (the chord is removed)

## REMOVED Requirements

### Requirement: ge chord for GoEnd in Profile/Template
**Reason**: Replaced by single `G` key for consistency with Connections tab and vim conventions.
**Migration**: Users with custom keymap.yaml entries mapping `ge` to GoEnd must update to `G`. The `ge` chord is no longer recognized in profile and template tabs.

### Requirement: Single d key for Profile delete
**Reason**: Replaced by `dd` chord with confirmation dialog to prevent accidental deletion.
**Migration**: Users with custom keymap.yaml entries mapping single `d` to Delete must update to `d d` chord.
