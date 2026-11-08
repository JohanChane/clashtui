use std::time::Instant;

use crate::tui::event::Event;
use crate::tui::monitor::CoreMonitor;
use crate::tui::tab::TuiTab;
use crate::tui::tab::prelude::*;

pub(in crate::tui::app) struct Hub {
    /// Order matches TabTarget::ALL.
    tabs: Vec<Tab>,
    shown: usize,
    monitor: CoreMonitor,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            tabs: vec![
                StatusTab::default().into(),
                FileTab::default().into(),
                ProxiesTab::default().into(),
                ConnectionsTab::default().into(),
                LogsTab::default().into(),
                SettingsTab::default().into(),
                CoreSrvCtlTab::default().into(),
            ],
            shown: 0,
            monitor: CoreMonitor::start(),
        }
    }

    /// Deliver Init to every tab, then Shown to the first tab.
    pub fn start(&mut self) {
        for tab in &mut self.tabs {
            tab.deliver(&Event::Init);
        }
        self.tabs[self.shown].deliver(&Event::Shown);
    }

    /// Deliver Hidden to the old tab and Shown to the new tab.
    pub fn show(&mut self, index: usize) {
        if index == self.shown {
            return;
        }
        self.tabs[self.shown].deliver(&Event::Hidden);
        self.shown = index;
        self.tabs[self.shown].deliver(&Event::Shown);
    }

    /// Broadcast to CoreMonitor first, then the visible tab. Hidden tabs reload on Shown.
    pub fn dispatch(&mut self, event: &Event) {
        debug_assert!(matches!(event, Event::Core(_) | Event::File(_)));
        self.monitor.on_event(event);
        self.tabs[self.shown].deliver(event);
    }

    /// Each frame, all tabs apply callbacks and visible pages check Tick.
    pub fn sync(&mut self) {
        let now = Instant::now();
        for tab in &mut self.tabs {
            tab.sync(now);
        }
    }

    pub fn shown(&mut self) -> &mut Tab {
        &mut self.tabs[self.shown]
    }

    pub fn shown_index(&self) -> usize {
        self.shown
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn titles(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.tabs.iter().map(TuiTab::title)
    }
}
