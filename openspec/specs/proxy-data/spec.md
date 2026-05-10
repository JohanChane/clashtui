# proxy-data Specification

## Purpose
TBD - created by archiving change add-proxies-tab. Update Purpose after archive.
## Requirements
### Requirement: Fetch all proxies from Mihomo API
The system SHALL fetch proxy data from Mihomo's `/proxies` endpoint and deserialize it into a structured `ProxiesResponse` containing a map of `Proxy` objects keyed by name.

#### Scenario: Successful fetch
- **WHEN** the TUI app starts and ProxiesTab becomes active
- **THEN** the system makes a GET request to `{external-controller}/proxies` with Bearer auth if configured
- **AND** deserializes the JSON response into `HashMap<String, Proxy>`

#### Scenario: Fetch with network error
- **WHEN** the Mihomo API is unreachable
- **THEN** the system SHALL catch the error via the `tri!` macro
- **AND** set an error message on the ProxiesTab content to display in the UI

#### Scenario: Fetch with empty proxy list
- **WHEN** Mihomo returns an empty proxies map
- **THEN** the UI SHALL display "No proxies found" message

### Requirement: Build tree structure from flat proxy data
The system SHALL construct a hierarchical `ProxyTree` from the flat proxy map by identifying root nodes (proxies not referenced by any `all` field of another proxy) and recursively building child nodes from each proxy group's `all` list.

#### Scenario: Simple selector group with child nodes
- **WHEN** `/proxies` returns: `GLOBAL: {type: Selector, all: ["Entry", "DIRECT"]}`, `Entry: {type: Selector, all: ["node1"]}`, `node1: {type: Vmess}`, `DIRECT: {type: Direct}`
- **THEN** the tree SHALL have root `GLOBAL` with children `Entry` and `DIRECT`
- **AND** `Entry` SHALL have child `node1`
- **AND** `node1` and `DIRECT` SHALL have no children (leaf nodes)

#### Scenario: Proxies not referenced by any group become roots
- **WHEN** a proxy exists but no other proxy's `all` field references it
- **THEN** that proxy SHALL appear as a top-level root node in the tree

#### Scenario: Hidden proxies are excluded
- **WHEN** a proxy has `hidden: true`
- **THEN** it SHALL NOT appear in the tree

### Requirement: Display proxy tree in TUI with file browser metaphor
The system SHALL render the proxy tree as a ratatui `List` widget using a file browser metaphor: proxy groups as folders, leaf nodes as files. Indentation represents nesting depth. Visual markers indicate type and expansion state.

#### Scenario: Render collapsed folder (proxy group)
- **WHEN** a proxy group (Selector/URLTest/Fallback/LoadBalance) is collapsed
- **THEN** it SHALL display a `▶` prefix and NOT show its children

#### Scenario: Render expanded folder (proxy group)
- **WHEN** a proxy group is expanded
- **THEN** it SHALL display a `▼` prefix and show its children indented

#### Scenario: Render file (leaf node)
- **WHEN** a node has no children (e.g., Vmess, Direct, Reject, Shadowsocks)
- **THEN** it SHALL display its name with indentation but no expand/collapse marker

#### Scenario: Render delay information
- **WHEN** a node has cached delay data
- **THEN** the delay in milliseconds SHALL appear at the end of the line (e.g., `231ms`)

#### Scenario: Render loading indicator
- **WHEN** a node has a pending speed test transaction
- **THEN** the node line SHALL show a rotating animation character (`-/|\`) before the name

### Requirement: Periodic auto-refresh of proxy data
The system SHALL automatically refresh proxy data from the API every 5 seconds while ProxiesTab is active, using the `after_sync` hook pattern.

#### Scenario: Periodic refresh triggers
- **WHEN** ProxiesTab is the active tab and 5 seconds have elapsed since the last refresh
- **THEN** the system SHALL fetch `/proxies` and rebuild the tree
- **AND** preserve the expansion state of existing tree nodes

### Requirement: Navigate proxy tree with keyboard
The system SHALL support standard Vim-like keyboard navigation within the proxy tree.

#### Scenario: Basic up/down navigation
- **WHEN** user presses `j` or `↓`
- **THEN** the selection cursor SHALL move to the next visible tree item
- **AND** wrapping from last to first item

#### Scenario: Expand node
- **WHEN** user presses `Enter`, `→`, or `l` on a collapsed group
- **THEN** the group SHALL expand to show its children

#### Scenario: Collapse folder
- **WHEN** user presses `←` or `h` on an expanded group
- **THEN** the group SHALL collapse, hiding its children

#### Scenario: Collapse parent folder from file
- **WHEN** user presses `u` while cursor is on a leaf node (file)
- **THEN** the system SHALL find the nearest ancestor folder in the tree
- **AND** collapse that folder, hiding all its children including the current file

