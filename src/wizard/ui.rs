use super::draft::{self, Document, Result, Target};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use serde_yaml::Value;
use std::io::{self, IsTerminal};

const STEPS: [&str; 6] = [
    "Project",
    "Agents",
    "Registries",
    "Skills",
    "Workspace",
    "Review",
];

/// Terminal roles use the user's palette and default background for both themes.
struct Theme;
impl Theme {
    fn title() -> Style {
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    }
    fn selected() -> Style {
        Style::default().add_modifier(Modifier::REVERSED)
    }
    fn muted() -> Style {
        Style::default().add_modifier(Modifier::DIM)
    }
    fn error() -> Style {
        Style::default().fg(Color::Red)
    }
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let guard = Self;
        execute!(
            io::stderr(),
            terminal::EnterAlternateScreen,
            event::EnableBracketedPaste
        )?;
        Ok(guard)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn restore_terminal() {
    let _ = execute!(
        io::stderr(),
        event::DisableBracketedPaste,
        terminal::LeaveAlternateScreen,
        cursor::Show
    );
    let _ = terminal::disable_raw_mode();
}

pub fn run_wizard(document: &mut Document, global: bool) -> Result<bool> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err("The init wizard needs a terminal on stdin and stderr; use 'skm init --non-interactive' for scripts".into());
    }
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous_hook(info);
    }));
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
    let mut app = App::new(global);
    loop {
        terminal.draw(|frame| render(frame, &mut app, document))?;
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match app.key(document, key) {
                Action::Continue => {}
                Action::Cancel => return Ok(false),
                Action::Save => match document.save(global) {
                    Ok(_) => return Ok(true),
                    Err(error) => app.error = error.to_string(),
                },
            },
            Event::Paste(text) => app.paste(document, &text),
            _ => {}
        }
    }
}

#[derive(Debug, PartialEq)]
enum Action {
    Continue,
    Save,
    Cancel,
}

struct Edit {
    target: Target,
    original: Value,
    text: String,
    cursor: usize, // byte offset, always on a UTF-8 boundary
}

struct App {
    step: usize,
    selected: usize,
    list: ListState,
    preview_scroll: u16,
    preview_horizontal: u16,
    edit: Option<Edit>,
    remove: Option<Target>,
    error: String,
    global: bool,
    usable_size: bool,
}

impl App {
    fn new(global: bool) -> Self {
        Self {
            step: 0,
            selected: 0,
            list: ListState::default(),
            preview_scroll: 0,
            preview_horizontal: 0,
            edit: None,
            remove: None,
            error: String::new(),
            global,
            usable_size: true,
        }
    }

    fn key(&mut self, document: &mut Document, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Cancel;
        }
        if !self.usable_size {
            return if key.code == KeyCode::Esc {
                Action::Cancel
            } else {
                Action::Continue
            };
        }
        if self.edit.is_some() {
            self.edit_key(document, key);
            return Action::Continue;
        }
        if let Some(target) = &self.remove {
            if key.code == KeyCode::Char('y') {
                draft::remove_entry(&mut document.value, target);
                self.selected = self.selected.min(
                    draft::fields(&document.value, self.step)
                        .len()
                        .saturating_sub(1),
                );
            }
            if matches!(key.code, KeyCode::Char('y' | 'n') | KeyCode::Esc) {
                self.remove = None;
            }
            return Action::Continue;
        }
        let fields = draft::fields(&document.value, self.step);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Action::Cancel,
            KeyCode::Tab | KeyCode::Right => self.change_step(1),
            KeyCode::BackTab | KeyCode::Left => self.change_step(-1),
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(fields.len().saturating_sub(1))
            }
            KeyCode::PageDown => {
                let last = document
                    .preview()
                    .unwrap_or_default()
                    .lines()
                    .count()
                    .saturating_sub(1)
                    .min(u16::MAX as usize) as u16;
                self.preview_scroll = self.preview_scroll.saturating_add(8).min(last);
            }
            KeyCode::PageUp => self.preview_scroll = self.preview_scroll.saturating_sub(8),
            KeyCode::Char(']') => {
                self.preview_horizontal = self.preview_horizontal.saturating_add(8)
            }
            KeyCode::Char('[') => {
                self.preview_horizontal = self.preview_horizontal.saturating_sub(8)
            }
            KeyCode::Char('a') if matches!(self.step, 2 | 3) => {
                self.selected = draft::add_entry(&mut document.value, self.step);
                self.error.clear();
            }
            KeyCode::Delete | KeyCode::Char('d') if matches!(self.step, 2 | 3) => {
                self.remove = fields.get(self.selected).map(|f| f.target.clone());
            }
            KeyCode::Char('s') | KeyCode::Enter if self.step == 5 => {
                match document.validate(self.global) {
                    Ok(()) => return Action::Save,
                    Err(error) => self.error = error.to_string(),
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(field) = fields.get(self.selected) {
                    self.error.clear();
                    if matches!(field.target, Target::Agent(_)) {
                        if let Err(error) = field.target.write(&mut document.value, "") {
                            self.error = error.to_string();
                        }
                    } else if key.code == KeyCode::Enter {
                        let text = field.target.read(&document.value);
                        self.edit = Some(Edit {
                            target: field.target.clone(),
                            original: document.value.clone(),
                            cursor: text.len(),
                            text,
                        });
                    }
                }
            }
            _ => {}
        }
        Action::Continue
    }

    fn change_step(&mut self, direction: isize) {
        self.step = self
            .step
            .saturating_add_signed(direction)
            .min(STEPS.len() - 1);
        self.selected = 0;
        self.list = ListState::default();
        self.error.clear();
    }

    fn edit_key(&mut self, document: &mut Document, key: KeyEvent) {
        let edit = self.edit.as_mut().unwrap();
        match key.code {
            KeyCode::Esc => {
                document.value = edit.original.clone();
                self.edit = None;
                self.error.clear();
                return;
            }
            KeyCode::Enter => {
                if self.error.is_empty() {
                    self.edit = None;
                }
                return;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                edit.text.clear();
                edit.cursor = 0;
            }
            KeyCode::Char(character)
                if !character.is_control()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                edit.text.insert(edit.cursor, character);
                edit.cursor += character.len_utf8();
            }
            KeyCode::Backspace if edit.cursor > 0 => {
                let previous = edit.text[..edit.cursor]
                    .char_indices()
                    .next_back()
                    .unwrap()
                    .0;
                edit.text.drain(previous..edit.cursor);
                edit.cursor = previous;
            }
            KeyCode::Delete if edit.cursor < edit.text.len() => {
                edit.text.remove(edit.cursor);
            }
            KeyCode::Left if edit.cursor > 0 => {
                edit.cursor = edit.text[..edit.cursor]
                    .char_indices()
                    .next_back()
                    .unwrap()
                    .0;
            }
            KeyCode::Right if edit.cursor < edit.text.len() => {
                edit.cursor += edit.text[edit.cursor..].chars().next().unwrap().len_utf8();
            }
            KeyCode::Home => edit.cursor = 0,
            KeyCode::End => edit.cursor = edit.text.len(),
            _ => {}
        }
        self.update_edit(document);
    }

    fn paste(&mut self, document: &mut Document, text: &str) {
        if let Some(edit) = &mut self.edit {
            let text: String = text.chars().filter(|c| !c.is_control()).collect();
            edit.text.insert_str(edit.cursor, &text);
            edit.cursor += text.len();
            self.update_edit(document);
        }
    }

    fn update_edit(&mut self, document: &mut Document) {
        let edit = self.edit.as_ref().unwrap();
        let mut candidate = edit.original.clone();
        match edit.target.write(&mut candidate, &edit.text) {
            Ok(()) => {
                document.value = candidate;
                self.error.clear();
            }
            Err(error) => self.error = error.to_string(),
        }
    }
}

fn render(frame: &mut Frame, app: &mut App, document: &Document) {
    let area = frame.area();
    app.usable_size = area.width >= 48 && area.height >= 20;
    if !app.usable_size {
        frame.render_widget(
            Paragraph::new(
                "Enlarge terminal to at least 48 x 20.\nEsc / Ctrl-C: cancel without saving.",
            ),
            area,
        );
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(5),
    ])
    .split(area);
    let state = if document.exists() {
        "Existing file"
    } else {
        "New configuration"
    };
    let scope = if app.global {
        "global installation"
    } else {
        "project installation"
    };
    let heading = vec![
        Line::styled(
            format!(" SKM setup  |  {state}  |  {scope}"),
            Theme::title(),
        ),
        Line::from(format!(
            " Step {}/{}: {}  |  {}",
            app.step + 1,
            STEPS.len(),
            STEPS[app.step],
            if document.changed() {
                "Unsaved draft"
            } else {
                "Unchanged"
            }
        )),
    ];
    frame.render_widget(Paragraph::new(heading), rows[0]);
    let panels = if area.width >= 100 {
        Layout::horizontal([Constraint::Percentage(46), Constraint::Percentage(54)]).split(rows[1])
    } else {
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1])
    };
    render_form(frame, app, document, panels[0]);
    let preview = document.preview().unwrap_or_else(|e| e.to_string());
    let line_count = preview.lines().count();
    app.preview_scroll = app
        .preview_scroll
        .min(line_count.saturating_sub(1).min(u16::MAX as usize) as u16);
    frame.render_widget(
        Paragraph::new(preview)
            .block(Block::bordered().title(format!(
                " skills.yaml | line {}/{} ",
                app.preview_scroll + 1,
                line_count
            )))
            .scroll((app.preview_scroll, app.preview_horizontal)),
        panels[1],
    );
    let (hints, actions) = if app.edit.is_some() {
        (
            "Enter: keep edit  Esc: undo edit",
            "Ctrl-U: clear  Ctrl-C: cancel wizard",
        )
    } else if app.step == 5 {
        (
            "Enter / s: save  Shift-Tab: back",
            "Esc / Ctrl-C: cancel wizard",
        )
    } else {
        (
            "Tab: next  Shift-Tab: back  Esc: cancel",
            match app.step {
                1 => "Up/Down: select  Space / Enter: toggle",
                2 | 3 => "Enter: edit  Up/Down: select  a: add  d: remove",
                _ => "Up/Down: select  Enter: edit",
            },
        )
    };
    let detail = if app.error.is_empty() {
        draft::fields(&document.value, app.step)
            .get(app.selected)
            .map(|f| f.hint)
            .unwrap_or("Review the preview before saving skills.yaml.")
            .to_owned()
    } else {
        format!("Error: {}", app.error)
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                detail,
                if app.error.is_empty() {
                    Theme::muted()
                } else {
                    Theme::error()
                },
            ),
            Line::from(hints),
            Line::from(actions),
            Line::from("PgUp/PgDn: scroll YAML  [/]: pan YAML"),
        ]),
        rows[2],
    );
    if app.remove.is_some() {
        let popup = centered(area, 48, 5);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new("Remove this entry from the draft?\n\ny: remove    n / Esc: keep")
                .block(Block::bordered().title(" Confirm removal ")),
            popup,
        );
    }
}

fn render_form(frame: &mut Frame, app: &mut App, document: &Document, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", STEPS[app.step]));
    if app.step == 5 {
        let validation = match document.validate(app.global) {
            Ok(()) => "Ready to save. Press Enter to save skills.yaml.".to_owned(),
            Err(error) => format!("Needs attention: {error}\n\nUse Shift-Tab to return and edit."),
        };
        let text = format!("{validation}\n\nFile: {}\n\n{}\n\nEdits preserve configuration values. YAML comments and formatting may be normalized.\n\nAfter saving, run skm install{} to apply the configuration.", document.path.display(), if document.changed() { "The preview shows the contents to be saved." } else { "No changes. Original formatting will be kept." }, if app.global { " --global" } else { "" });
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: false }).block(block),
            area,
        );
        return;
    }
    let fields = draft::fields(&document.value, app.step);
    if fields.is_empty() {
        frame.render_widget(
            Paragraph::new("No entries yet.\n\nPress a to add an entry, or Tab to continue.")
                .wrap(Wrap { trim: false })
                .block(block),
            area,
        );
        return;
    }
    let items: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let value = if i == app.selected {
                if let Some(edit) = &app.edit {
                    // Keep the insertion point in view without splitting a Unicode character.
                    let mut before = &edit.text[..edit.cursor];
                    while Line::from(before).width() > usize::from(area.width.saturating_sub(8)) {
                        before = &before[before.chars().next().unwrap().len_utf8()..];
                    }
                    format!("{}|{}", before, &edit.text[edit.cursor..])
                } else {
                    field.target.read(&document.value)
                }
            } else {
                field.target.read(&document.value)
            };
            ListItem::new(vec![
                Line::from(field.label.clone()),
                Line::from(Span::styled(
                    format!("  {}", if value.is_empty() { "(empty)" } else { &value }),
                    Theme::muted(),
                )),
            ])
        })
        .collect();
    app.list.select(Some(app.selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(Theme::selected())
            .highlight_symbol("> "),
        area,
        &mut app.list,
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use std::fs;

    fn setup() -> (tempfile::TempDir, Document, App) {
        let dir = tempfile::tempdir().unwrap();
        let document =
            Document::load(&dir.path().join("skills.yaml"), Some("demo"), false).unwrap();
        (dir, document, App::new(false))
    }

    fn press(app: &mut App, document: &mut Document, code: KeyCode) -> Action {
        app.key(document, KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn screen(app: &mut App, document: &Document, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, app, document)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn navigation_preserves_edits_and_save_is_only_available_at_review() {
        let (_dir, mut document, mut app) = setup();
        assert_eq!(
            press(&mut app, &mut document, KeyCode::Char('s')),
            Action::Continue
        );
        press(&mut app, &mut document, KeyCode::Enter);
        app.paste(&mut document, "-edited");
        assert!(document.preview().unwrap().contains("demo-edited"));
        assert!(!document.path.exists());
        press(&mut app, &mut document, KeyCode::Enter);
        for _ in 0..5 {
            press(&mut app, &mut document, KeyCode::Tab);
        }
        assert_eq!(app.step, 5);
        press(&mut app, &mut document, KeyCode::BackTab);
        assert_eq!(app.step, 4);
        press(&mut app, &mut document, KeyCode::Tab);
        assert_eq!(press(&mut app, &mut document, KeyCode::Enter), Action::Save);
        assert_eq!(document.value["name"], "demo-edited");
        // The controller never writes the file itself.
        assert!(!document.path.exists());
    }

    #[test]
    fn undo_edit_and_cancel_leave_existing_bytes_untouched() {
        let (_dir, mut document, mut app) = setup();
        document.save(false).unwrap();
        let original = fs::read_to_string(&document.path).unwrap();
        document = Document::load(&document.path, None, false).unwrap();
        press(&mut app, &mut document, KeyCode::Enter);
        app.paste(&mut document, "-unsaved");
        press(&mut app, &mut document, KeyCode::Esc);
        assert_eq!(document.preview().unwrap(), original);
        assert_eq!(press(&mut app, &mut document, KeyCode::Esc), Action::Cancel);
        assert_eq!(fs::read_to_string(&document.path).unwrap(), original);
        press(&mut app, &mut document, KeyCode::Enter);
        app.paste(&mut document, "-aborted");
        assert_eq!(
            app.key(
                &mut document,
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
            ),
            Action::Cancel
        );
        assert_eq!(fs::read_to_string(&document.path).unwrap(), original);
    }

    #[test]
    fn unicode_editing_cursor_and_paste_keep_valid_text() {
        let (_dir, mut document, mut app) = setup();
        press(&mut app, &mut document, KeyCode::Enter);
        app.key(
            &mut document,
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        app.paste(&mut document, "é界\n\t");
        press(&mut app, &mut document, KeyCode::Left);
        press(&mut app, &mut document, KeyCode::Backspace);
        press(&mut app, &mut document, KeyCode::Char('ö'));
        press(&mut app, &mut document, KeyCode::Delete);
        assert_eq!(document.value["name"], "ö");
        press(&mut app, &mut document, KeyCode::Home);
        press(&mut app, &mut document, KeyCode::Char('界'));
        press(&mut app, &mut document, KeyCode::End);
        press(&mut app, &mut document, KeyCode::Backspace);
        assert_eq!(document.value["name"], "界");
        press(&mut app, &mut document, KeyCode::Enter);
        assert!(app.edit.is_none());
    }

    #[test]
    fn deletion_needs_confirmation_and_invalid_review_blocks_save() {
        let (_dir, mut document, mut app) = setup();
        app.step = 3;
        press(&mut app, &mut document, KeyCode::Char('a'));
        let incomplete = document.value.clone();
        app.step = 5;
        assert_eq!(
            press(&mut app, &mut document, KeyCode::Enter),
            Action::Continue
        );
        assert!(!app.error.is_empty());
        app.step = 3;
        press(&mut app, &mut document, KeyCode::Char('d'));
        assert!(screen(&mut app, &document, 100, 30).contains("Confirm removal"));
        press(&mut app, &mut document, KeyCode::Char('n'));
        assert_eq!(document.value, incomplete);
        press(&mut app, &mut document, KeyCode::Char('d'));
        press(&mut app, &mut document, KeyCode::Char('y'));
        assert!(document.value["skills"].as_sequence().unwrap().is_empty());
        assert!(!document.path.exists());
    }

    #[test]
    fn renderer_shows_loaded_and_live_yaml_in_wide_and_stacked_layouts() {
        let (_dir, mut document, mut app) = setup();
        for (width, height) in [(120, 32), (80, 24), (48, 20)] {
            let output = screen(&mut app, &document, width, height);
            assert!(output.contains("New configuration"));
            assert!(output.contains("Project name"));
            assert!(output.contains("skills.yaml"));
            assert!(output.contains("name: demo"));
            assert!(output.contains("Tab: next"));
        }
        document.save(false).unwrap();
        document = Document::load(&document.path, None, false).unwrap();
        assert!(screen(&mut app, &document, 100, 30).contains("Existing file"));
        press(&mut app, &mut document, KeyCode::Enter);
        app.paste(&mut document, "-new");
        assert!(screen(&mut app, &document, 100, 30).contains("name: demo-new"));
    }

    #[test]
    fn small_terminal_cannot_save_blindly_and_recovers_after_resize() {
        let (_dir, mut document, mut app) = setup();
        app.step = 5;
        assert!(screen(&mut app, &document, 40, 12).contains("Enlarge terminal"));
        assert_eq!(
            press(&mut app, &mut document, KeyCode::Enter),
            Action::Continue
        );
        assert!(!document.path.exists());
        screen(&mut app, &document, 100, 30);
        assert_eq!(press(&mut app, &mut document, KeyCode::Enter), Action::Save);
        screen(&mut app, &document, 40, 12);
        assert_eq!(press(&mut app, &mut document, KeyCode::Esc), Action::Cancel);
    }

    #[test]
    fn lists_and_preview_scroll_to_later_entries() {
        let (_dir, mut document, mut app) = setup();
        app.step = 3;
        for i in 0..20 {
            draft::add_entry(&mut document.value, 3);
            Target::Skill(i, "name")
                .write(&mut document.value, &format!("skill-{i}"))
                .unwrap();
        }
        for _ in 0..79 {
            press(&mut app, &mut document, KeyCode::Down);
        }
        let output = screen(&mut app, &document, 100, 30);
        assert!(output.contains("Skill 20 name"));
        for _ in 0..100 {
            press(&mut app, &mut document, KeyCode::PageDown);
        }
        assert!(screen(&mut app, &document, 100, 30).contains("version: latest"));
        let previous = app.preview_scroll;
        press(&mut app, &mut document, KeyCode::PageUp);
        assert!(app.preview_scroll < previous);
    }
}
