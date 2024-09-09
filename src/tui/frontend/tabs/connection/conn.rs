use super::*;
#[derive(Clone)]
pub struct Connection {
    chains: String,
    domain: String,
    rule_type: String,
    start: String,
    upload: u64,
    download: u64,
    pub id: Option<String>,
}
impl Connection {
    pub fn build_col(&self) -> Raw::Row {
        use Ra::Text;
        Raw::Row::new([
            Text::from(self.domain.as_str()).centered(),
            Text::from(self.chains.as_str()).centered(),
            Text::from(self.rule_type.to_string()).centered(),
            Text::from(self.start.as_str()).centered(),
            Text::from(bytes_to_readable(self.upload, None)).centered(),
            Text::from(bytes_to_readable(self.download, None)).centered(),
        ])
    }
    /// once lose track, this [Connection] will never be tracked again
    pub fn lose_track(mut self) -> Box<Self> {
        self.id = None;
        Box::new(self)
    }
    pub fn build_header() -> Raw::Row<'static> {
        const BAR: [&str; 6] = ["Url", "Chain", "Type", "Start Time", "Recvived", "Send"];
        Raw::Row::new(BAR.into_iter().map(|s| Ra::Text::from(s).centered()))
    }
    pub fn build_footer(upload: u64, download: u64) -> Raw::Row<'static> {
        use Ra::Text;
        Raw::Row::new([
            Text::from("Total").centered(),
            Text::from("").centered(),
            Text::from("").centered(),
            Text::from("").centered(),
            Text::from(bytes_to_readable(upload, None)).centered(),
            Text::from(bytes_to_readable(download, None)).centered(),
        ])
    }
}

impl From<Conn> for Connection {
    fn from(value: Conn) -> Self {
        let Conn {
            id,
            metadata,
            upload,
            download,
            start,
            mut chains,
        } = value;
        let ConnMetaData {
            network: _,
            ctype,
            host,
            process: _,
            process_path: _,
            source_ip: _,
            source_port: _,
            remote_destination: _,
            destinatio_port: _,
        } = metadata;
        let mut chain = String::with_capacity(1024);
        if let Some(c) = chains.pop() {
            chain.push_str(&c);
            while let Some(c) = chains.pop() {
                chain.push_str(" -> ");
                chain.push_str(&c);
            }
        }
        Self {
            chains: chain,
            domain: host,
            rule_type: ctype,
            start,
            upload,
            download,
            id: Some(id),
        }
    }
}

#[test]
#[cfg(test)]
#[ignore = "used to preview widget"]
fn t() {
    fn df(f: &mut Ra::Frame) {
        let this = Connection {
            chains: "name".to_owned(),
            domain: "domain".to_owned(),
            rule_type: "Unknown".to_string(),
            start: "start time".to_owned(),
            upload: 100,
            download: 10000,
            id: Some("id".to_owned()),
        };
        f.render_widget(&this, f.area())
    }
    // let o = std::panic::take_hook();
    // std::panic::set_hook(Box::new(move |i| {
    //     let _ = setup::restore();
    //     o(i)
    // }));
    // use crate::tui::setup;
    // setup::setup().unwrap();
    let mut terminal = Ra::Terminal::new(Ra::CrosstermBackend::new(std::io::stdout())).unwrap();
    terminal.draw(|f| df(f)).unwrap();
    // std::thread::sleep(std::time::Duration::new(5, 0));
    // setup::restore().unwrap();
}

impl Raw::WidgetRef for Connection {
    /// draw like
    ///```md
    /// ┌───────────────────────────────────┬───────────┐
    /// │domain                             │↑upload    │
    /// ├───────────────────────────────────┼───────────┤
    /// │chain                              │↓download  │
    /// ├──────────────────────┬────────────┴───────────┤
    /// │Type                  │start time              │
    /// ├──────────────────────┴────────────────────────┤
    /// │Id                                             │
    /// ├───────────────────────────────────────────────┤
    /// │Prompt                                         │
    /// └───────────────────────────────────────────────┘
    ///```
    fn render_ref(&self, area: Ra::Rect, buf: &mut Ra::Buffer)
    where
        Self: Sized,
    {
        use Ra::{
            symbols::{border, line::NORMAL},
            Constraint,
            Constraint::Length,
            Layout, Stylize,
        };
        use Raw::{Block, Borders, Paragraph};

        // 5 rows, offset = 10
        let a_centered =
            tools::centered_rect(Constraint::Percentage(60), Constraint::Length(11), area);
        Raw::Clear.render(a_centered, buf);

        let hes = Layout::vertical([Length(3), Length(1), Length(3), Length(1), Length(3)])
            .split(a_centered);
        let rc_r0 = Layout::horizontal([Constraint::Percentage(100), Constraint::Min(10 + 1 + 2)])
            .split(hes[0]);
        let a_domain = rc_r0[0];
        // ┌────────────────────────────────────
        // │domain
        // ├────────────────────────────────────
        let b_domain = Block::new()
            .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
            .border_set(border::Set {
                bottom_left: NORMAL.vertical_right,
                ..border::PLAIN
            });
        let a_upload = rc_r0[1];
        // ┬──────────┐
        // │upload    │
        // ┼──────────┤
        let b_upload = Block::new()
            .borders(Borders::all())
            .border_set(border::Set {
                top_left: NORMAL.horizontal_down,
                bottom_left: NORMAL.cross,
                bottom_right: NORMAL.vertical_left,
                ..border::PLAIN
            });
        let rc_r1 = Layout::horizontal([Constraint::Percentage(100), Constraint::Min(10 + 1 + 2)])
            .split(hes[1]);
        let a_chain = rc_r1[0];
        //
        // │chain
        //
        let b_chain = Block::new().borders(Borders::LEFT);
        let a_download = rc_r1[1];
        //
        // │download  │
        //
        let b_download = Block::new().borders(Borders::LEFT | Borders::RIGHT);
        let rc_r2 = Layout::horizontal([
            Constraint::Percentage(100),
            Constraint::Min(24 - 10 - 1),
            Constraint::Min(10 + 1 + 2),
        ])
        .split(hes[2]);
        let a_type = rc_r2[0];
        // ├──────────────────────
        // │Type
        // ├──────────────────────
        let b_type = Block::new()
            .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
            .border_set(border::Set {
                top_left: NORMAL.vertical_right,
                bottom_left: NORMAL.vertical_right,
                ..border::PLAIN
            });
        let a_startime_0 = rc_r2[1];
        // ┬─────────────
        // │start time
        // ┴─────────────
        let b_startime_0 = Block::new()
            .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
            .border_set(border::Set {
                top_left: NORMAL.horizontal_down,
                bottom_left: NORMAL.horizontal_up,
                ..border::PLAIN
            });
        let a_startime_1 = rc_r2[2];
        // ┴──────────┤
        //            │
        // ───────────┤
        let b_startime_1 = Block::new()
            .borders(Borders::all())
            .border_set(border::Set {
                vertical_left: " ",
                top_left: NORMAL.horizontal_up,
                bottom_left: NORMAL.horizontal,
                top_right: NORMAL.vertical_left,
                bottom_right: NORMAL.vertical_left,
                ..border::PLAIN
            });
        let a_id = hes[3];
        //
        // │Id                                             │
        //
        let b_id = Block::new().borders(Borders::LEFT | Borders::RIGHT);
        let a_promopt = hes[4];
        // ├───────────────────────────────────────────────┤
        // │Prompt                                         │
        // └───────────────────────────────────────────────┘
        let b_promopt = Block::new()
            .borders(Borders::all())
            .border_set(border::Set {
                top_left: NORMAL.vertical_right,
                top_right: NORMAL.vertical_left,
                ..border::PLAIN
            });
        use Ra::Widget;
        Paragraph::new(self.domain.as_str())
            .block(b_domain.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_domain, buf);
        Paragraph::new(self.rule_type.as_str())
            .block(b_type.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_type, buf);
        Paragraph::new(self.chains.as_str())
            .block(b_chain.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_chain, buf);
        Paragraph::new(self.start.chars().take(14).collect::<String>())
            .block(b_startime_0.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_startime_0, buf);
        Paragraph::new(self.start.chars().skip(14).collect::<String>())
            .block(b_startime_1.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_startime_1, buf);
        Paragraph::new(bytes_to_readable(self.upload, Some("↑")))
            .block(b_upload.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_upload, buf);
        Paragraph::new(bytes_to_readable(self.download, Some("↓")))
            .block(b_download.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_download, buf);
        Paragraph::new(self.id.as_deref().unwrap_or("Lose Track, Maybe Closed"))
            .block(b_id.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_id, buf);
        Paragraph::new("Press Enter to terminate this connection, Esc to close")
            .block(b_promopt.fg(Theme::get().popup_block_fg))
            .fg(Theme::get().popup_text_fg)
            .render(a_promopt, buf);
    }
}

fn bytes_to_readable(bytes: u64, prefix: Option<&str>) -> String {
    const KILOBYTE: u64 = 1024;
    const MEGABYTE: u64 = KILOBYTE * 1024;
    const GIGABYTE: u64 = MEGABYTE * 1024;
    const TERABYTE: u64 = GIGABYTE * 1024;

    let prefix = prefix.unwrap_or_default();

    if bytes >= TERABYTE {
        format!("{prefix}{:.2} TB", bytes as f64 / TERABYTE as f64)
    } else if bytes >= GIGABYTE {
        format!("{prefix}{:.2} GB", bytes as f64 / GIGABYTE as f64)
    } else if bytes >= MEGABYTE {
        format!("{prefix}{:.2} MB", bytes as f64 / MEGABYTE as f64)
    } else if bytes >= KILOBYTE {
        format!("{prefix}{:.2} KB", bytes as f64 / KILOBYTE as f64)
    } else {
        format!("{prefix}{} Bytes", bytes)
    }
}
