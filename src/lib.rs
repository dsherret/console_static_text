use ansi::strip_ansi_codes;
use word::tokenize_words;
use word::WordToken;

mod ansi;
mod word;

const VTS_MOVE_TO_ZERO_COL: &str = "\x1B[0G";
const VTS_CLEAR_CURSOR_DOWN: &str = concat!(
  "\x1B[2K", // clear current line
  "\x1B[J",  // clear cursor down
);
const VTS_CLEAR_UNTIL_NEWLINE: &str = "\x1B[K";

fn vts_move_up(count: usize) -> String {
  if count == 0 {
    String::new()
  } else {
    format!("\x1B[{}A", count)
  }
}

fn vts_move_down(count: usize) -> String {
  if count == 0 {
    String::new()
  } else {
    format!("\x1B[{}B", count)
  }
}

pub enum TextItem<'a> {
  Text(&'a str),
  HangingText { text: &'a str, indent: u16 },
}

impl<'a> TextItem<'a> {
  pub fn new(text: &'a str) -> Self {
    Self::Text(text)
  }

  pub fn with_hanging_indent(text: &'a str, indent: u16) -> Self {
    Self::HangingText { text, indent }
  }
}

#[derive(Debug, PartialEq, Eq)]
struct Line {
  pub char_width: usize,
  pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleSize {
  pub cols: Option<u16>,
  pub rows: Option<u16>,
}

pub struct ConsoleStaticText {
  console_size: Box<dyn (Fn() -> ConsoleSize) + Send + 'static>,
  last_lines: Vec<Line>,
  last_size: ConsoleSize,
}

impl std::fmt::Debug for ConsoleStaticText {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("StaticText")
      .field("last_lines", &self.last_lines)
      .field("last_size", &self.last_size)
      .finish()
  }
}

impl ConsoleStaticText {
  pub fn new(
    console_size: impl (Fn() -> ConsoleSize) + Send + 'static,
  ) -> Self {
    Self {
      console_size: Box::new(console_size),
      last_lines: Vec::new(),
      last_size: ConsoleSize {
        cols: None,
        rows: None,
      },
    }
  }

  pub fn eprint_clear(&mut self) {
    if let Some(text) = self.render_clear() {
      eprint!("{}", text);
    }
  }

  pub fn render_clear(&mut self) -> Option<String> {
    let size = (self.console_size)();
    let last_lines = self.get_last_lines(size);
    if !last_lines.is_empty() {
      let mut text = VTS_MOVE_TO_ZERO_COL.to_string();
      let move_up_count = last_lines.len() - 1;
      if move_up_count > 0 {
        text.push_str(&vts_move_up(move_up_count));
      }
      text.push_str(VTS_CLEAR_CURSOR_DOWN);
      Some(text)
    } else {
      None
    }
  }

  pub fn eprint(&mut self, new_text: &str) {
    if let Some(text) = self.render(new_text) {
      eprint!("{}", text);
    }
  }

  pub fn eprint_with_size(&mut self, new_text: &str, size: ConsoleSize) {
    if let Some(text) = self.render_with_size(new_text, size) {
      eprint!("{}", text);
    }
  }

  pub fn render(&mut self, new_text: &str) -> Option<String> {
    self.render_with_size(new_text, (self.console_size)())
  }

  pub fn render_with_size(
    &mut self,
    new_text: &str,
    size: ConsoleSize,
  ) -> Option<String> {
    self.render_with_size_from_items(
      vec![TextItem::Text(new_text)].into_iter(),
      size,
    )
  }

  pub fn render_with_size_from_items<'a>(
    &mut self,
    text_items: impl Iterator<Item = TextItem<'a>>,
    size: ConsoleSize,
  ) -> Option<String> {
    let is_terminal_different_size = size != self.last_size;
    let last_lines = self.get_last_lines(size);
    let new_lines = render_items(text_items, size);
    let last_lines_for_new_lines = self.raw_render_last_items(
      &new_lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n"),
      size,
    );
    let result =
      if !are_collections_equal(&last_lines, &last_lines_for_new_lines) {
        let mut text = String::new();
        text.push_str(VTS_MOVE_TO_ZERO_COL);
        if last_lines.len() > 1 {
          text.push_str(&vts_move_up(last_lines.len() - 1));
        }
        if is_terminal_different_size {
          text.push_str(VTS_CLEAR_CURSOR_DOWN);
        }
        for (i, new_line) in new_lines.iter().enumerate() {
          if i > 0 {
            text.push('\n');
          }
          text.push_str(&new_line.text);
          if !is_terminal_different_size {
            if let Some(last_line) = last_lines.get(i) {
              if last_line.char_width > new_line.char_width {
                text.push_str(VTS_CLEAR_UNTIL_NEWLINE);
              }
            }
          }
        }
        if last_lines.len() > new_lines.len() {
          text.push_str(&vts_move_down(1));
          text.push_str(VTS_CLEAR_CURSOR_DOWN);
          text.push_str(&vts_move_up(1));
        }
        Some(text)
      } else {
        None
      };
    self.last_lines = last_lines_for_new_lines;
    self.last_size = size;
    result
  }

  fn get_last_lines(&mut self, size: ConsoleSize) -> Vec<Line> {
    // render based on how the text looks right now
    let size = ConsoleSize {
      cols: std::cmp::min(size.cols, self.last_size.cols),
      rows: std::cmp::min(size.rows, self.last_size.rows),
    };

    if size == self.last_size {
      self.last_lines.drain(..).collect()
    } else {
      // render the last text with the current terminal width
      let line_texts = self
        .last_lines
        .drain(..)
        .map(|l| l.text)
        .collect::<Vec<_>>();
      let text = line_texts.join("\n");
      self.raw_render_last_items(&text, size)
    }
  }

  fn raw_render_last_items(&self, text: &str, size: ConsoleSize) -> Vec<Line> {
    let mut lines = Vec::new();
    let text = strip_ansi_codes(text);
    if let Some(terminal_width) = size.cols.map(|c| c as usize) {
      for line in text.split('\n') {
        let mut count = 0;
        let mut current_line = String::new();
        for c in line.chars() {
          if let Some(width) = unicode_width::UnicodeWidthChar::width(c) {
            if count + width > terminal_width {
              lines.push(Line {
                char_width: count,
                text: current_line,
              });
              current_line = c.to_string();
              count = width;
            } else {
              count += width;
              current_line.push(c);
            }
          }
        }
        if !current_line.is_empty() {
          lines.push(Line {
            char_width: count,
            text: current_line,
          });
        }
      }
    } else {
      for line in text.split('\n') {
        lines.push(Line {
          char_width: text_width(line),
          text: line.to_string(),
        });
      }
    }
    truncate_lines_height(lines, size)
  }
}

fn render_items<'a>(
  text_items: impl Iterator<Item = TextItem<'a>>,
  size: ConsoleSize,
) -> Vec<Line> {
  let mut lines = Vec::new();
  let terminal_width = size.cols.map(|c| c as usize);
  for item in text_items {
    match item {
      TextItem::Text(text) => {
        lines.extend(render_text_to_lines(text, 0, terminal_width))
      }
      TextItem::HangingText { text, indent } => {
        lines.extend(render_text_to_lines(
          text,
          indent as usize,
          terminal_width,
        ));
      }
    }
  }

  truncate_lines_height(lines, size)
}

fn truncate_lines_height(lines: Vec<Line>, size: ConsoleSize) -> Vec<Line> {
  match size.rows.map(|c| c as usize) {
    Some(terminal_height) if lines.len() > terminal_height => {
      let cutoff_index = lines.len() - terminal_height;
      lines
        .into_iter()
        .enumerate()
        .filter_map(|(index, line)| {
          if index < cutoff_index {
            None
          } else {
            Some(line)
          }
        })
        .collect()
    }
    _ => lines,
  }
}

fn render_text_to_lines(
  text: &str,
  hanging_indent: usize,
  terminal_width: Option<usize>,
) -> Vec<Line> {
  let mut lines = Vec::new();
  if let Some(terminal_width) = terminal_width {
    let mut current_line = String::new();
    let mut line_width = 0;
    let mut current_whitespace = String::new();
    for token in tokenize_words(text) {
      match token {
        WordToken::Word(word) => {
          let word_width = text_width(&strip_ansi_codes(word));
          let is_word_longer_than_half_line =
            hanging_indent + word_width > (terminal_width / 2);
          if is_word_longer_than_half_line {
            // break it up onto multiple lines with indentation if able
            if !current_whitespace.is_empty() {
              if line_width < terminal_width {
                current_line.push_str(&current_whitespace);
              }
              current_whitespace = String::new();
            }
            for ansi_token in ansi::tokenize(word) {
              if ansi_token.is_escape {
                current_line.push_str(&word[ansi_token.range]);
              } else {
                for c in word[ansi_token.range].chars() {
                  if let Some(char_width) =
                    unicode_width::UnicodeWidthChar::width(c)
                  {
                    if line_width + char_width > terminal_width {
                      lines.push(Line {
                        char_width: line_width,
                        text: current_line,
                      });
                      current_line = String::new();
                      current_line.push_str(&" ".repeat(hanging_indent));
                      line_width = hanging_indent;
                    }
                    current_line.push(c);
                    line_width += char_width;
                  } else {
                    current_line.push(c);
                  }
                }
              }
            }
          } else {
            if line_width + word_width > terminal_width {
              lines.push(Line {
                char_width: line_width,
                text: current_line,
              });
              current_line = String::new();
              current_line.push_str(&" ".repeat(hanging_indent));
              line_width = hanging_indent;
              current_whitespace = String::new();
            }
            if !current_whitespace.is_empty() {
              current_line.push_str(&current_whitespace);
              current_whitespace = String::new();
            }
            current_line.push_str(word);
            line_width += word_width;
          }
        }
        WordToken::WhiteSpace(space_char) => {
          current_whitespace.push(space_char);
          line_width += 1;
        }
        WordToken::NewLine => {
          lines.push(Line {
            char_width: line_width,
            text: current_line,
          });
          current_line = String::new();
          line_width = 0;
        }
      }
    }
    if !current_line.is_empty() {
      lines.push(Line {
        char_width: line_width,
        text: current_line,
      });
    }
  } else {
    for line in text.split('\n') {
      lines.push(Line {
        char_width: text_width(&strip_ansi_codes(line)),
        text: line.to_string(),
      });
    }
  }
  lines
}

fn text_width(text: &str) -> usize {
  unicode_width::UnicodeWidthStr::width(text)
}

fn are_collections_equal<T: PartialEq>(a: &[T], b: &[T]) -> bool {
  a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a == b)
}

#[cfg(test)]
mod test {
  use crate::ansi::strip_ansi_codes;
  use crate::vts_move_down;
  use crate::vts_move_up;
  use crate::ConsoleSize;
  use crate::ConsoleStaticText;
  use crate::VTS_CLEAR_CURSOR_DOWN;
  use crate::VTS_CLEAR_UNTIL_NEWLINE;
  use crate::VTS_MOVE_TO_ZERO_COL;

  #[test]
  fn renders() {
    let mut text = create();
    let result = text.render("01234567890123456").unwrap();
    assert_eq!(strip_ansi_codes(&result), "0123456789\n0123456");
    let result = text.render("123").unwrap();
    assert_eq!(
      result,
      format!(
        "{}{}{}{}{}{}{}",
        VTS_MOVE_TO_ZERO_COL,
        vts_move_up(1),
        "123",
        VTS_CLEAR_UNTIL_NEWLINE,
        vts_move_down(1),
        VTS_CLEAR_CURSOR_DOWN,
        vts_move_up(1),
      )
    );

    let mut text = create();
    let result = text.render("1").unwrap();
    assert_eq!(strip_ansi_codes(&result), "1");
    let result = text.render("").unwrap();
    assert_eq!(strip_ansi_codes(&result), "");
  }

  #[test]
  fn moves_long_text_multiple_lines() {
    let mut text = create();
    let result = text.render("012345 67890").unwrap();
    assert_eq!(strip_ansi_codes(&result), "012345\n67890");
    let result = text.render("01234567890 67890").unwrap();
    assert_eq!(strip_ansi_codes(&result), "0123456789\n0 67890");
  }

  fn create() -> ConsoleStaticText {
    ConsoleStaticText::new(|| ConsoleSize {
      cols: Some(10),
      rows: Some(10),
    })
  }
}
