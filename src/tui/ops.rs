//! Shared operations that modify the core or files. They emit events on success; callers need not emit.
//! Service commands emit ServiceCommandDone on completion, whether they succeed or fail.
//! Use the domain modules directly. Callers handle user interaction and task scheduling.

pub(in crate::tui) mod config;
pub(in crate::tui) mod connections;
pub(in crate::tui) mod profile;
pub(in crate::tui) mod service;
