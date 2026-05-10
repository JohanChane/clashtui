# signal-handling Specification

## Purpose
TBD - created by archiving change migrate-yazi-key-modifiers-and-signals. Update Purpose after archive.
## Requirements
### Requirement: Signal handler spawns a tokio task with biased select

The system SHALL spawn a `Signals` tokio task at startup that listens on three event sources using `tokio::select!` with biased priority: (1) an mpsc channel for stop/resume control, (2) a `signal_hook_tokio` stream for OS signals, (3) a crossterm `EventStream` for terminal key events. The task SHALL be spawned from `Signals::start()` and its handle owned by the main event loop.

#### Scenario: Signal task runs concurrently with main loop
- **WHEN** `Signals::start()` is called during TUI init
- **THEN** a tokio task is spawned and begins listening on the three event sources

#### Scenario: Biased select prioritizes control over signals over terminal events
- **WHEN** a stop/resume message and an OS signal arrive simultaneously
- **THEN** the stop/resume message is processed first due to biased priority

### Requirement: SIGINT is ignored at the OS level

The signal handler SHALL explicitly match on `SIGINT` and take no action. The signal SHALL NOT cause a quit event. Ctrl-C SHALL be received as a keyboard event (`KeyCode::Char('c')` with `ctrl = true`) through the crossterm EventStream because the terminal is in raw mode (`enable_raw_mode()` disables ISIG).

#### Scenario: Ctrl-C keypress in raw mode
- **WHEN** user presses Ctrl-C while the TUI is in raw mode
- **THEN** the OS does NOT deliver SIGINT to the process; crossterm delivers `KeyEvent { code: Char('c'), modifiers: CONTROL }`

#### Scenario: kill -INT from outside
- **WHEN** the process receives SIGINT via `kill -INT <pid>` from another process
- **THEN** the signal handler ignores the signal and the TUI continues running

### Requirement: SIGQUIT, SIGHUP, and SIGTERM trigger graceful quit

The signal handler SHALL emit `Event::Quit` when `SIGQUIT`, `SIGHUP`, or `SIGTERM` is received. The signal task SHALL exit its loop after emitting the quit event. The main event loop SHALL process the Quit event, restore the terminal, and call `std::process::exit`.

#### Scenario: SIGTERM quits gracefully
- **WHEN** the process receives SIGTERM
- **THEN** `Event::Quit` is emitted to the global channel, the signal task exits, the main loop dispatches the quit event, the terminal is restored, and the process exits

#### Scenario: SIGHUP quits gracefully
- **WHEN** the process receives SIGHUP (terminal closed)
- **THEN** `Event::Quit` is emitted and the process exits after terminal restoration

### Requirement: SIGTSTP suspends and SIGCONT resumes

The signal handler SHALL, on `SIGTSTP`: (1) call stop logic to restore the terminal and drop the EventStream, (2) send `SIGSTOP` to the process. On `SIGCONT`: the signal handler SHALL (3) re-enable raw mode and create a new EventStream, (4) trigger a full render.

#### Scenario: Ctrl-Z suspends the process
- **WHEN** user presses Ctrl-Z
- **THEN** the terminal is restored (raw mode disabled, alt screen left), the terminal EventStream is dropped, and the process is stopped via SIGSTOP

#### Scenario: fg resumes the process
- **WHEN** the stopped process receives SIGCONT (via `fg` or `kill -CONT`)
- **THEN** raw mode is re-enabled, a new EventStream is created, and a full render is triggered

### Requirement: Signals module provides stop/resume control API

The `Signals` struct SHALL expose `stop()` and `resume()` methods that send control messages over the internal mpsc channel to the spawned task. `stop()` SHALL cause the task to drop the `EventStream` (suspending terminal event processing). `resume()` SHALL cause the task to create a new `EventStream` and resume processing.

#### Scenario: App calls Signals::stop()
- **WHEN** `signals.stop()` is called
- **THEN** the spawned task receives the stop message, drops the current EventStream, and the biased select skips the terminal event arm

#### Scenario: App calls Signals::resume()
- **WHEN** `signals.resume()` is called after a stop
- **THEN** the spawned task creates a new EventStream and resumes processing terminal events

### Requirement: Signal handling is Unix-only

Signal registration SHALL use `#[cfg(unix)]` to compile only on Unix platforms. On Windows, `Signals::start()` SHALL create an empty sys stream via `tokio_stream::empty()` and return a valid `Signals` instance. The `EventStream` arm SHALL still function normally on Windows.

#### Scenario: Signal module compiles on Windows
- **WHEN** building on Windows
- **THEN** `signals::spawn()` compiles without error using `tokio_stream::empty()` for the signal source

