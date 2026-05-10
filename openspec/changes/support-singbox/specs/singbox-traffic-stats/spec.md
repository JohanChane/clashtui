# singbox-traffic-stats Specification

## Purpose
Define the WebSocket-based traffic statistics collection for sing-box, and how it normalizes to demotui's internal traffic model.

## ADDED Requirements

### Requirement: WebSocket traffic client
The system SHALL implement a WebSocket client that connects to sing-box's `/traffic` endpoint at `ws://{controller}/traffic`, parses the JSON message stream, and computes speed deltas between messages.

#### Scenario: Successful WebSocket connection
- **WHEN** sing-box is running and serving the `clash_api`
- **THEN** the traffic client SHALL establish a WebSocket connection to `ws://{controller}/traffic`

#### Scenario: Parse traffic message
- **WHEN** the WebSocket receives `{"up": 123456789, "down": 987654321}`
- **THEN** the client SHALL parse `up` and `down` as cumulative byte counters (u64)

#### Scenario: Compute speed delta
- **WHEN** two consecutive messages are received with up values 100 and 200
- **THEN** the computed upload speed SHALL be 100 bytes per interval

### Requirement: Traffic stats normalization
The system SHALL normalize sing-box WebSocket traffic data into the same internal `TrafficStats` representation used by mihomo's poll-based traffic. The TUI display SHALL be core-type-agnostic.

#### Scenario: Normalized traffic struct
- **WHEN** traffic data arrives from either mihomo (poll) or sing-box (WebSocket)
- **THEN** both SHALL populate a shared `TrafficStats { total_up: u64, total_down: u64, speed_up: u64, speed_down: u64 }` struct

#### Scenario: No proxy/direct split
- **WHEN** sing-box provides only total up/down (no proxy/direct split)
- **THEN** the proxy-specific fields SHALL be set to 0 or the same as total

### Requirement: WebSocket reconnection
The system SHALL automatically reconnect the WebSocket when the connection is lost, with exponential backoff up to a maximum delay.

#### Scenario: Connection dropped
- **WHEN** the WebSocket connection to sing-box is lost
- **THEN** the client SHALL attempt to reconnect after 1 second, then 2 seconds, then 4 seconds, up to a maximum of 30 seconds

#### Scenario: Singbox not running
- **WHEN** the WebSocket client cannot connect (sing-box not running)
- **THEN** the client SHALL retry periodically without blocking the TUI event loop
- **AND** traffic display SHALL show zero or the last known values

### Requirement: Traffic stats lifecycle
The WebSocket traffic client SHALL be started when sing-box becomes the active core and stopped when demotui exits or core type changes.

#### Scenario: Start traffic client on singbox activation
- **WHEN** `core_type` is `"singbox"` and the sing-box service is started
- **THEN** the WebSocket traffic client SHALL connect and begin receiving data

#### Scenario: Stop traffic client on exit
- **WHEN** demotui is shutting down
- **THEN** the WebSocket connection SHALL be closed gracefully
