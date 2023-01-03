use std::io::stderr;

use console_static_text::ConsoleSize;
use console_static_text::ConsoleStaticText;
use console_static_text::TextItem;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::execute;

struct DrawState {
  active_index: usize,
  message: String,
  items: Vec<String>,
}

pub fn main() {
  assert!(crossterm::tty::IsTty::is_tty(&std::io::stderr()));
  let mut static_text = ConsoleStaticText::new(|| console_size());
  let mut state = DrawState {
    active_index: 0,
    message: "Which option would you like to select?".to_string(),
    items: vec![
      "Option 1".to_string(),
      "Option 2".to_string(),
      "Option 3 with long text. ".repeat(10),
      "Option 4".to_string(),
    ],
  };

  // enable raw mode to get special key presses
  crossterm::terminal::enable_raw_mode().unwrap();
  // hide the cursor
  execute!(stderr(), crossterm::cursor::Hide).unwrap();

  // render, then act on up and down arrow key presses
  loop {
    let items = render(&state);
    static_text.eprint_items(items.iter());

    if let Event::Key(event) = event::read().unwrap() {
      // in a real implementation you will want to handle ctrl+c here
      // (make sure to handle always turning off raw mode)
      match event {
        KeyEvent {
          code: KeyCode::Up, ..
        } => {
          if state.active_index == 0 {
            state.active_index = state.items.len() - 1;
          } else {
            state.active_index -= 1;
          }
        }
        KeyEvent {
          code: KeyCode::Down,
          ..
        } => {
          state.active_index = (state.active_index + 1) % state.items.len();
        }
        KeyEvent {
          code: KeyCode::Enter,
          ..
        } => {
          break;
        }
        _ => {
          // ignore
        }
      }
    };
  }

  // disable raw mode, show the cursor, clear the static text, then
  // display what the user selected
  crossterm::terminal::disable_raw_mode().unwrap();
  execute!(stderr(), crossterm::cursor::Show).unwrap();
  static_text.eprint_clear();
  eprintln!("Selected: {}", state.items[state.active_index]);
}

fn console_size() -> ConsoleSize {
  // get the size from crossterm and don't bother with the
  // "sized" feature in order to reduce our dependencies
  let (cols, rows) = crossterm::terminal::size().unwrap();
  ConsoleSize {
    rows: Some(rows),
    cols: Some(cols),
  }
}

/// Renders the draw state
fn render(state: &DrawState) -> Vec<TextItem> {
  let mut items = Vec::new();

  // display the question message
  items.push(TextItem::new(&state.message));

  // now render each item, showing a `>` beside the active index
  for (i, item_text) in state.items.iter().enumerate() {
    let selection_char = if i == state.active_index { '>' } else { ' ' };
    let text = format!("{} {}", selection_char, item_text);
    items.push(TextItem::HangingText {
      text: std::borrow::Cow::Owned(text),
      indent: 4,
    });
  }

  items
}
