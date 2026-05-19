use std::{cell::RefCell, rc::Rc};

use ratzilla::{
    DomBackend, WebRenderer,
    event::{KeyCode, KeyEvent},
    ratatui::{
        Frame, Terminal,
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Paragraph, Wrap},
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum View {
    Overview,
    Blocks,
    Operators,
    Work,
}

#[derive(Debug)]
struct ExplorerTuiState {
    view: View,
    refreshes: u64,
}

impl ExplorerTuiState {
    fn next_view(&mut self) {
        self.view = match self.view {
            View::Overview => View::Blocks,
            View::Blocks => View::Operators,
            View::Operators => View::Work,
            View::Work => View::Overview,
        };
    }

    fn previous_view(&mut self) {
        self.view = match self.view {
            View::Overview => View::Work,
            View::Blocks => View::Overview,
            View::Operators => View::Blocks,
            View::Work => View::Operators,
        };
    }

    fn view_name(&self) -> &'static str {
        match self.view {
            View::Overview => "overview",
            View::Blocks => "blocks",
            View::Operators => "operators",
            View::Work => "work",
        }
    }
}

pub fn run() -> std::io::Result<()> {
    let backend = DomBackend::new()?;
    let terminal = Terminal::new(backend)?;
    let state = Rc::new(RefCell::new(ExplorerTuiState {
        view: View::Overview,
        refreshes: 0,
    }));
    let event_state = Rc::clone(&state);
    terminal.on_key_event(move |event| {
        handle_key(event, &mut event_state.borrow_mut());
    });
    terminal.draw_web(move |frame| render(frame, &state));
    Ok(())
}

fn handle_key(event: KeyEvent, state: &mut ExplorerTuiState) {
    match event.code {
        KeyCode::Left | KeyCode::Up => state.previous_view(),
        KeyCode::Right | KeyCode::Down | KeyCode::Tab => state.next_view(),
        _ => {}
    }
}

fn render(frame: &mut Frame<'_>, state: &Rc<RefCell<ExplorerTuiState>>) {
    let mut state = state.borrow_mut();
    state.refreshes += 1;
    let area = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(40)])
        .split(vertical[2]);

    let title = Paragraph::new("TensorVM Explorer")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("local testnet"),
        );
    frame.render_widget(title, vertical[0]);

    let tabs = Paragraph::new("overview  blocks  operators  work").block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("view: {}", state.view_name())),
    );
    frame.render_widget(tabs, vertical[1]);

    let side = Paragraph::new(format!(
        "height      live\nminers      live\nvalidators  live\nreceipts    live\nframes      {}",
        state.refreshes
    ))
    .block(Block::default().borders(Borders::ALL).title("summary"));
    frame.render_widget(side, body[0]);

    let detail = Paragraph::new(view_body(state.view))
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title("chain data"));
    frame.render_widget(detail, body[1]);

    let footer = Paragraph::new("ws /explorer/ws | default ui | ratzilla")
        .block(Block::default().borders(Borders::ALL).title("status"));
    frame.render_widget(footer, vertical[3]);
}

fn view_body(view: View) -> &'static str {
    match view {
        View::Overview => "latest block height, epoch, receipts, jobs, rewards",
        View::Blocks => "height | epoch | hash | proposer | state root | time",
        View::Operators => "miners and validators with stake, work, reputation, rewards",
        View::Work => "settled receipts and current tensor jobs",
    }
}
