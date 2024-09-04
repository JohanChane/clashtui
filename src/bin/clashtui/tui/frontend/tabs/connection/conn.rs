use super::*;
#[derive(Clone)]
pub struct Connection {
    name: String,
    domain: String,
    rule_type: String,
    start: String,
    upload: u64,
    download: u64,
    pub id: String,
}
impl Connection {
    pub fn build_col(&self) -> Raw::Row {
        use Ra::{Alignment, Text};
        Raw::Row::new([
            Text::from(self.name.as_str()).alignment(Alignment::Left),
            Text::from(self.domain.as_str()).alignment(Alignment::Left),
            Text::from(self.rule_type.to_string()).alignment(Alignment::Center),
            Text::from(self.start.as_str()).alignment(Alignment::Left),
            Text::from(self.upload.to_string()).alignment(Alignment::Center),
            Text::from(self.download.to_string()).alignment(Alignment::Center),
        ])
    }
    /// once lose track, this [Connection] will never be tracked again
    pub fn lose_track(mut self) -> Box<Self> {
        self.id = "Lose Track, Maybe Closed".to_owned();
        Box::new(self)
    }
    pub fn build_header() -> Raw::Row<'static> {
        const BAR: [&'static str; 6] = ["Name", "Url", "Type", "Start Time", "Recvived", "Send"];
        Raw::Row::new(BAR.into_iter().map(|s| Ra::Text::from(s).centered()))
    }
    pub fn build_footer(upload: u64, download: u64) -> Raw::Row<'static> {
        use Ra::Text;
        Raw::Row::new([
            Text::from("Total").centered(),
            Text::from("").centered(),
            Text::from("").centered(),
            Text::from("").centered(),
            Text::from(bytes_to_readable(upload)).centered(),
            Text::from(bytes_to_readable(download)).centered(),
        ])
    }
}

impl From<clashtui::webapi::Conn> for Connection {
    fn from(value: clashtui::webapi::Conn) -> Self {
        let clashtui::webapi::Conn {
            id,
            metadata,
            upload,
            download,
            start,
            chains,
        } = value;
        let clashtui::webapi::ConnMetaData {
            network,
            ctype,
            host,
            process,
            process_path,
            source_ip,
            source_port,
            remote_destination,
            destinatio_port,
        } = metadata;
        Self {
            name: process,
            domain: host,
            rule_type: ctype,
            start,
            upload,
            download,
            id,
        }
    }
}

#[test]
fn t() {
    fn df(f: &mut Ra::Frame) {
        let this = Connection {
            name: "name".to_owned(),
            domain: "domain".to_owned(),
            rule_type: "Unknown".to_string(),
            start: "start time".to_owned(),
            upload: 100,
            download: 10000,
            id: "id".to_owned(),
        };
        f.render_widget(this, f.area())
    }
    use crate::tui::setup;
    setup::setup().unwrap();
    let mut terminal = Ra::Terminal::new(Ra::CrosstermBackend::new(std::io::stdout())).unwrap();
    terminal.draw(|f| df(f)).unwrap();
    std::thread::sleep(std::time::Duration::new(5, 0));
    setup::restore().unwrap();
}

impl Ra::Widget for Connection {
    /// draw like
    ///
    /// |col1|col2|
    /// |---:|:---|
    /// |name|type|
    /// |domain|
    /// |upload|download|
    /// |start|
    /// |id|
    fn render(self, area: Ra::Rect, buf: &mut Ra::Buffer)
    where
        Self: Sized,
    {
        use Ra::{
            symbols::{border, line::NORMAL},
            Constraint,
            Constraint::Ratio,
            Direction, Layout, Rect,
        };
        use Raw::{Block, Borders, Paragraph};
        fn collapse(
            constraints: Vec<Constraint>,
            rf_blocks: Option<(border::Set, Borders)>,
            area: Rect,
            direction: Direction,
        ) -> (Vec<(border::Set, Borders)>, Vec<Rect>) {
            let blocks: Vec<(border::Set, Borders)> = vec![
                (
                    Default::default(),
                    rf_blocks.map(|b| b.1).unwrap_or(Borders::ALL)
                );
                constraints.len()
            ];
            let mut modified_aera = area.clone();
            let modified_aera = match direction {
                Direction::Horizontal => {
                    modified_aera.width -= 1;
                    modified_aera
                }
                Direction::Vertical => {
                    modified_aera.height -= 1;
                    modified_aera
                }
            };
            let mut rects = Layout::new(direction, constraints)
                .split(modified_aera)
                .to_vec();
            let last_rect = rects.last_mut().unwrap();
            match direction {
                Direction::Horizontal => last_rect.width += 1,
                Direction::Vertical => last_rect.height += 1,
            }
            let mut blocks: Vec<(border::Set, Borders)> = blocks
                .into_iter()
                .map(|(fontset, border)| match direction {
                    Direction::Horizontal => (
                        border::Set {
                            bottom_left: NORMAL.horizontal_up,
                            top_left: if fontset.top_left != NORMAL.vertical_right {
                                NORMAL.horizontal_down
                            } else {
                                NORMAL.cross
                            },
                            ..fontset
                        },
                        border & (Borders::TOP | Borders::BOTTOM | Borders::LEFT),
                    ),
                    Direction::Vertical => (
                        border::Set {
                            top_right: NORMAL.vertical_left,
                            top_left: if fontset.top_left != NORMAL.horizontal_down {
                                NORMAL.vertical_right
                            } else {
                                NORMAL.cross
                            },
                            ..fontset
                        },
                        border & (Borders::TOP | Borders::RIGHT | Borders::LEFT),
                    ),
                })
                .collect();
            let first_block = blocks.first_mut().unwrap();
            first_block.0 = rf_blocks.unwrap_or_default().0;
            let last_block = blocks.last_mut().unwrap();
            match direction {
                Direction::Horizontal => last_block.1 |= Borders::RIGHT,
                Direction::Vertical => last_block.1 |= Borders::BOTTOM,
            }
            (blocks, rects)
        }

        let rows = collapse(vec![Ratio(1, 5); 5], None, area, Direction::Vertical);
        let name_type = collapse(
            vec![Ratio(1, 2); 2],
            Some(rows.0[0]),
            rows.1[0],
            Direction::Horizontal,
        );
        let domain = collapse(
            vec![Ratio(1, 1); 1],
            Some(rows.0[1]),
            rows.1[1],
            Direction::Horizontal,
        );
        let up_download = collapse(
            vec![Ratio(1, 2); 2],
            Some(rows.0[2]),
            rows.1[2],
            Direction::Horizontal,
        );
        let start = collapse(
            vec![Ratio(1, 1); 1],
            Some(rows.0[3]),
            rows.1[3],
            Direction::Horizontal,
        );
        let id = collapse(
            vec![Ratio(1, 1); 1],
            Some(rows.0[4]),
            rows.1[4],
            Direction::Horizontal,
        );

        // let rows = Layout::vertical([Ratio(1, 5); 5]).split(area);
        // let name_type = Layout::horizontal([Ratio(1, 2); 2]).split(rows[0]);
        // let domain = rows[1];
        // let up_download = Layout::horizontal([Ratio(1, 2); 2]).split(rows[2]);
        // let start = rows[3];
        // let id = rows[4];

        // ┌──
        // │
        // ├──
        Paragraph::new(self.name)
            .block(
                Block::new()
                    .border_set(name_type.0[0].0)
                    .borders(name_type.0[0].1),
            )
            .render(name_type.1[0], buf);
        // ┬──┐
        // │  │
        // ┴──┤
        Paragraph::new(self.rule_type.to_string())
            .block(
                Block::new()
                    .border_set(name_type.0[1].0)
                    .borders(name_type.0[1].1),
            )
            .render(name_type.1[1], buf);
        // │  │
        // │  │
        Paragraph::new(self.domain)
            .block(
                Block::new()
                    .border_set(domain.0[0].0)
                    .borders(domain.0[0].1),
            )
            .render(domain.1[0], buf);
        // ├──
        // │
        // ├──
        Paragraph::new(self.upload.to_string())
            .block(
                Block::new()
                    .border_set(up_download.0[0].0)
                    .borders(up_download.0[0].1),
            )
            .render(up_download.1[0], buf);
        // ┬──┤
        // │  │
        // ┴──┤
        Paragraph::new(self.download.to_string())
            .block(
                Block::new()
                    .border_set(up_download.0[1].0)
                    .borders(up_download.0[1].1),
            )
            .render(up_download.1[1], buf);
        // │  │
        // │  │
        Paragraph::new(self.start)
            .block(Block::new().border_set(start.0[0].0).borders(start.0[0].1))
            .render(start.1[0], buf);
        // ├──┤
        // │  │
        // └──┘
        Paragraph::new(self.id)
            .block(Block::new().border_set(id.0[0].0).borders(id.0[0].1))
            .render(id.1[0], buf);
    }
}

fn bytes_to_readable(bytes: u64) -> String {
    const KILOBYTE: u64 = 1024;
    const MEGABYTE: u64 = KILOBYTE * 1024;
    const GIGABYTE: u64 = MEGABYTE * 1024;
    const TERABYTE: u64 = GIGABYTE * 1024;

    if bytes >= TERABYTE {
        format!("{:.2} TB", bytes as f64 / TERABYTE as f64)
    } else if bytes >= GIGABYTE {
        format!("{:.2} GB", bytes as f64 / GIGABYTE as f64)
    } else if bytes >= MEGABYTE {
        format!("{:.2} MB", bytes as f64 / MEGABYTE as f64)
    } else if bytes >= KILOBYTE {
        format!("{:.2} KB", bytes as f64 / KILOBYTE as f64)
    } else {
        format!("{} Bytes", bytes)
    }
}
