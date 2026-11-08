use std::time::Instant;

use crate::tui::event::Event;
use crate::tui::monitor::CoreMonitor;
use crate::tui::tab::TuiTab;
use crate::tui::tab::prelude::*;

pub(in crate::tui::app) struct Hub {
    /// 顺序与 TabTarget::ALL 一致。
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

    /// 每个标签页收到 Init，然后第一个标签页收到 Shown。
    pub fn start(&mut self) {
        for tab in &mut self.tabs {
            tab.deliver(&Event::Init);
        }
        self.tabs[self.shown].deliver(&Event::Shown);
    }

    /// 旧标签页收到 Hidden，新标签页收到 Shown。
    pub fn show(&mut self, index: usize) {
        if index == self.shown {
            return;
        }
        self.tabs[self.shown].deliver(&Event::Hidden);
        self.shown = index;
        self.tabs[self.shown].deliver(&Event::Shown);
    }

    /// 广播：先 CoreMonitor，再按标签栏顺序投给每个标签页（包括隐藏的）。
    pub fn dispatch(&mut self, event: &Event) {
        debug_assert!(matches!(event, Event::Core(_) | Event::File(_)));
        self.monitor.on_event(event);
        for tab in &mut self.tabs {
            tab.deliver(event);
        }
    }

    /// 每帧：所有标签页应用回调；显示中的页面检查 Tick。
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
