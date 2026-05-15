use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

use crate::app::App;

pub fn run(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = main_loop(app, &mut terminal);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();
    result
}

fn main_loop<B: ratatui::backend::Backend>(
    app: &mut App,
    terminal: &mut Terminal<B>,
) -> Result<()> {
    while !app.should_quit {
        app.maybe_tick();
        terminal.draw(|f| draw(f, app))?;

        if event::poll(Duration::from_millis(33))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Char(' ') => app.paused = !app.paused,
                    KeyCode::Char('+') | KeyCode::Char('=') => app.speed_up(),
                    KeyCode::Char('-') | KeyCode::Char('_') => app.slow_down(),
                    KeyCode::Char('s') => {
                        if app.paused {
                            app.force_step();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn draw(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(34)])
        .split(f.area());

    draw_grid(f, chunks[0], app);
    draw_sidebar(f, chunks[1], app);
}

fn draw_grid(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let block = Block::default().title(" world ").borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let w = app.sim.grid.width as usize;
    let h = app.sim.grid.height as usize;

    // Build a per-cell render hint. Carrier > non-carrier > food > empty.
    #[derive(Clone, Copy)]
    enum CellRender {
        Empty,
        Food,
        Agent,
        Carrier,
    }
    let mut cells = vec![CellRender::Empty; w * h];

    for y in 0..h {
        for x in 0..w {
            if app.sim.grid.has_food(x as i32, y as i32) {
                cells[y * w + x] = CellRender::Food;
            }
        }
    }
    for a in &app.sim.agents {
        if !a.alive {
            continue;
        }
        let idx = a.position.y as usize * w + a.position.x as usize;
        let next = if a.meme.is_some() { CellRender::Carrier } else { CellRender::Agent };
        cells[idx] = match (cells[idx], next) {
            (CellRender::Carrier, _) => CellRender::Carrier,
            (_, n) => n,
        };
    }

    let visible_rows = (inner.height as usize).min(h);
    let visible_cols = (inner.width as usize).min(w);

    let mut lines: Vec<Line> = Vec::with_capacity(visible_rows);
    for y in 0..visible_rows {
        let mut spans: Vec<Span> = Vec::with_capacity(visible_cols);
        for x in 0..visible_cols {
            let (ch, style) = match cells[y * w + x] {
                CellRender::Empty => (' ', Style::default()),
                CellRender::Food => ('.', Style::default().fg(Color::Green)),
                CellRender::Agent => (
                    'a',
                    Style::default().fg(Color::White).add_modifier(Modifier::DIM),
                ),
                CellRender::Carrier => (
                    'A',
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                ),
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_sidebar(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let block = Block::default().title(" meme garden ").borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let m = app.last_metrics();
    let (tick, alive, food, carriers, prev, energy) = match m {
        Some(m) => (
            m.tick,
            m.alive,
            m.food_count,
            m.meme_carriers,
            m.meme_prevalence,
            m.mean_energy,
        ),
        None => (0, 0, 0, 0, 0.0, 0.0),
    };

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().add_modifier(Modifier::DIM);
    let carrier = Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(vec![
            Span::styled("tick     ", dim),
            Span::styled(format!("{tick}"), bold),
        ]),
        Line::from(vec![
            Span::styled("alive    ", dim),
            Span::styled(format!("{alive}"), bold),
        ]),
        Line::from(vec![
            Span::styled("food     ", dim),
            Span::styled(format!("{food}"), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("carriers ", dim),
            Span::styled(format!("{carriers}"), carrier),
        ]),
        Line::from(vec![
            Span::styled("prev.    ", dim),
            Span::styled(format!("{:.1}%", prev * 100.0), carrier),
        ]),
        Line::from(vec![
            Span::styled("mean E   ", dim),
            Span::styled(format!("{energy:.2}"), bold),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("tps      ", dim),
            Span::styled(format!("{:.1}", app.tps), bold),
        ]),
        Line::from(vec![
            Span::styled("state    ", dim),
            Span::styled(if app.paused { "paused" } else { "running" }, bold),
        ]),
        Line::from(""),
        Line::from(Span::styled("legend", bold)),
        Line::from(vec![
            Span::styled("A ", carrier),
            Span::raw("meme carrier"),
        ]),
        Line::from(vec![
            Span::styled("a ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("agent"),
        ]),
        Line::from(vec![
            Span::styled(". ", Style::default().fg(Color::Green)),
            Span::raw("food"),
        ]),
        Line::from(""),
        Line::from(Span::styled("keys", bold)),
        Line::from("space  pause/run"),
        Line::from("s      single step"),
        Line::from("+ / -  speed"),
        Line::from("q      quit"),
    ];

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
