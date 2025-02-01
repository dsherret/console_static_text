use std::borrow::Cow;
use std::io::Write;

use ansi::strip_ansi_codes;
use unicode_width::UnicodeWidthStr;
use word::tokenize_words;
use word::WordToken;

pub mod ansi;
#[cfg(feature = "sized")]
mod console;
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
  Text(Cow<'a, str>),
  HangingText { text: Cow<'a, str>, indent: u16 },
}

impl<'a> TextItem<'a> {
  pub fn new(text: &'a str) -> Self {
    Self::Text(Cow::Borrowed(text))
  }

  pub fn new_owned(text: String) -> Self {
    Self::Text(Cow::Owned(text))
  }

  pub fn with_hanging_indent(text: &'a str, indent: u16) -> Self {
    Self::HangingText {
      text: Cow::Borrowed(text),
      indent,
    }
  }

  pub fn with_hanging_indent_owned(text: String, indent: u16) -> Self {
    Self::HangingText {
      text: Cow::Owned(text),
      indent,
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
struct Line {
  pub char_width: usize,
  pub text: String,
}

impl Line {
  pub fn new(text: String) -> Self {
    Self {
      // measure the line width each time in order to not include trailing whitespace
      char_width: UnicodeWidthStr::width(strip_ansi_codes(&text).as_ref()),
      text,
    }
  }
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
  keep_cursor_zero_column: bool,
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
      keep_cursor_zero_column: true,
    }
  }

  /// Gets a `ConsoleStaticText` that knows how to get the console size.
  ///
  /// Returns `None` when stderr is not a tty or the console size can't be
  /// retrieved from stderr.
  #[cfg(feature = "sized")]
  pub fn new_sized() -> Option<Self> {
    if !atty::is(atty::Stream::Stderr) || console::size().is_none() {
      None
    } else {
      Some(Self::new(|| {
        let size = console::size();
        ConsoleSize {
          cols: size.map(|s| s.0 .0),
          rows: size.map(|s| s.1 .0),
        }
      }))
    }
  }

  /// Keeps the cursor at the zero column.
  pub fn keep_cursor_zero_column(&mut self, value: bool) {
    self.keep_cursor_zero_column = value;
  }

  pub fn console_size(&self) -> ConsoleSize {
    (self.console_size)()
  }

  pub fn eprint_clear(&mut self) {
    if let Some(text) = self.render_clear() {
      std::io::stderr().write_all(text.as_bytes()).unwrap();
    }
  }

  pub fn render_clear(&mut self) -> Option<String> {
    let size = self.console_size();
    self.render_clear_with_size(size)
  }

  pub fn render_clear_with_size(
    &mut self,
    size: ConsoleSize,
  ) -> Option<String> {
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
      std::io::stderr().write_all(text.as_bytes()).unwrap();
    }
  }

  pub fn eprint_with_size(&mut self, new_text: &str, size: ConsoleSize) {
    if let Some(text) = self.render_with_size(new_text, size) {
      std::io::stderr().write_all(text.as_bytes()).unwrap();
    }
  }

  pub fn render(&mut self, new_text: &str) -> Option<String> {
    self.render_with_size(new_text, self.console_size())
  }

  pub fn render_with_size(
    &mut self,
    new_text: &str,
    size: ConsoleSize,
  ) -> Option<String> {
    if new_text.is_empty() {
      self.render_clear_with_size(size)
    } else {
      self.render_items_with_size([TextItem::new(new_text)].iter(), size)
    }
  }

  pub fn eprint_items<'a>(
    &mut self,
    text_items: impl Iterator<Item = &'a TextItem<'a>>,
  ) {
    self.eprint_items_with_size(text_items, self.console_size())
  }

  pub fn eprint_items_with_size<'a>(
    &mut self,
    text_items: impl Iterator<Item = &'a TextItem<'a>>,
    size: ConsoleSize,
  ) {
    if let Some(text) = self.render_items_with_size(text_items, size) {
      std::io::stderr().write_all(text.as_bytes()).unwrap();
    }
  }

  pub fn render_items<'a>(
    &mut self,
    text_items: impl Iterator<Item = &'a TextItem<'a>>,
  ) -> Option<String> {
    self.render_items_with_size(text_items, self.console_size())
  }

  pub fn render_items_with_size<'a>(
    &mut self,
    text_items: impl Iterator<Item = &'a TextItem<'a>>,
    size: ConsoleSize,
  ) -> Option<String> {
    let is_terminal_different_size = size != self.last_size;
    let last_lines = self.get_last_lines(size);
    let new_lines = render_items(text_items, size);
    let last_lines_for_new_lines = raw_render_last_items(
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
            text.push_str("\r\n");
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
        if self.keep_cursor_zero_column {
          text.push_str(VTS_MOVE_TO_ZERO_COL);
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
      raw_render_last_items(&text, size)
    }
  }
}

fn raw_render_last_items(text: &str, size: ConsoleSize) -> Vec<Line> {
  let mut lines = Vec::new();
  let text = strip_ansi_codes(text);
  if let Some(terminal_width) = size.cols.map(|c| c as usize) {
    for line in text.split('\n') {
      if line.is_empty() {
        lines.push(Line::new(String::new()));
        continue;
      }
      let mut count = 0;
      let mut current_line = String::new();
      for c in line.chars() {
        if let Some(width) = unicode_width::UnicodeWidthChar::width(c) {
          if count + width > terminal_width {
            lines.push(Line::new(current_line));
            current_line = c.to_string();
            count = width;
          } else {
            count += width;
            current_line.push(c);
          }
        }
      }
      if !current_line.is_empty() {
        lines.push(Line::new(current_line));
      }
    }
  } else {
    for line in text.split('\n') {
      lines.push(Line::new(line.to_string()));
    }
  }
  truncate_lines_height(lines, size)
}

fn render_items<'a>(
  text_items: impl Iterator<Item = &'a TextItem<'a>>,
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
          *indent as usize,
          terminal_width,
        ));
      }
    }
  }

  let lines = truncate_lines_height(lines, size);
  // ensure there's always 1 line
  if lines.is_empty() {
    vec![Line::new(String::new())]
  } else {
    lines
  }
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
          let word_width =
            UnicodeWidthStr::width(strip_ansi_codes(word).as_ref());
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
                      lines.push(Line::new(current_line));
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
              lines.push(Line::new(current_line));
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
          line_width +=
            unicode_width::UnicodeWidthChar::width(space_char).unwrap_or(1);
        }
        WordToken::LfNewLine | WordToken::CrlfNewLine => {
          lines.push(Line::new(current_line));
          current_line = String::new();
          line_width = 0;
        }
      }
    }
    if !current_line.is_empty() {
      lines.push(Line::new(current_line));
    }
  } else {
    for line in text.split('\n') {
      lines.push(Line::new(line.to_string()));
    }
  }
  lines
}

fn are_collections_equal<T: PartialEq>(a: &[T], b: &[T]) -> bool {
  a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a == b)
}

#[cfg(test)]
mod test {
  use std::sync::Arc;
  use std::sync::Mutex;

  use crate::vts_move_down;
  use crate::vts_move_up;
  use crate::ConsoleSize;
  use crate::ConsoleStaticText;
  use crate::VTS_CLEAR_CURSOR_DOWN;
  use crate::VTS_CLEAR_UNTIL_NEWLINE;
  use crate::VTS_MOVE_TO_ZERO_COL;

  fn test_mappings() -> Vec<(String, String)> {
    let mut mappings = Vec::new();
    for i in 1..10 {
      mappings.push((format!("~CUP{}~", i), vts_move_up(i)));
      mappings.push((format!("~CDOWN{}~", i), vts_move_down(i)));
    }
    mappings.push((
      "~CLEAR_CDOWN~".to_string(),
      VTS_CLEAR_CURSOR_DOWN.to_string(),
    ));
    mappings.push((
      "~CLEAR_UNTIL_NEWLINE~".to_string(),
      VTS_CLEAR_UNTIL_NEWLINE.to_string(),
    ));
    mappings.push(("~MOVE0~".to_string(), VTS_MOVE_TO_ZERO_COL.to_string()));
    mappings
  }

  struct Tester {
    inner: ConsoleStaticText,
    size: Arc<Mutex<ConsoleSize>>,
    mappings: Vec<(String, String)>,
  }

  impl Tester {
    pub fn new() -> Self {
      let size = Arc::new(Mutex::new(ConsoleSize {
        cols: Some(10),
        rows: Some(10),
      }));
      Self {
        inner: {
          let size = size.clone();
          ConsoleStaticText::new(move || size.lock().unwrap().clone())
        },
        size,
        mappings: test_mappings(),
      }
    }

    pub fn set_cols(&self, cols: Option<u16>) {
      self.size.lock().unwrap().cols = cols;
    }

    pub fn set_rows(&self, rows: Option<u16>) {
      self.size.lock().unwrap().rows = rows;
    }

    /// Keeps the cursor displaying at the zero column (default).
    ///
    /// When set to `false`, this will keep the cursor at the end
    /// of the line.
    pub fn keep_cursor_zero_column(&mut self, value: bool) {
      self.inner.keep_cursor_zero_column(value);
    }

    pub fn render(&mut self, text: &str) -> Option<String> {
      self
        .inner
        .render(&self.map_text_to(text))
        .map(|text| self.map_text_from(&text))
    }

    pub fn render_clear(&mut self) -> Option<String> {
      self
        .inner
        .render_clear()
        .map(|text| self.map_text_from(&text))
    }

    fn map_text_to(&self, text: &str) -> String {
      let mut text = text.to_string();
      for (from, to) in &self.mappings {
        text = text.replace(from, to);
      }
      text
    }

    fn map_text_from(&self, text: &str) -> String {
      let mut text = text.to_string();
      for (to, from) in &self.mappings {
        text = text.replace(from, to);
      }
      text
    }
  }

  #[test]
  fn renders() {
    let mut tester = Tester::new();
    let result = tester.render("01234567890123456").unwrap();
    assert_eq!(result, "~MOVE0~~CLEAR_CDOWN~0123456789\r\n0123456~MOVE0~");
    let result = tester.render("123").unwrap();
    assert_eq!(
      result,
      "~MOVE0~~CUP1~123~CLEAR_UNTIL_NEWLINE~~CDOWN1~~CLEAR_CDOWN~~CUP1~~MOVE0~",
    );
    let result = tester.render_clear().unwrap();
    assert_eq!(result, "~MOVE0~~CLEAR_CDOWN~");

    let mut tester = Tester::new();
    let result = tester.render("1").unwrap();
    assert_eq!(result, "~MOVE0~~CLEAR_CDOWN~1~MOVE0~");
    let result = tester.render("").unwrap();
    assert_eq!(result, "~MOVE0~~CLEAR_CDOWN~");

    // should not add a move0 here
    tester.keep_cursor_zero_column(false);
    let result = tester.render("1").unwrap();
    assert_eq!(result, "~MOVE0~1");
  }

  #[test]
  fn moves_long_text_multiple_lines() {
    let mut tester = Tester::new();
    let result = tester.render("012345 67890").unwrap();
    assert_eq!(result, "~MOVE0~~CLEAR_CDOWN~012345\r\n67890~MOVE0~");
    let result = tester.render("01234567890 67890").unwrap();
    assert_eq!(result, "~MOVE0~~CUP1~0123456789\r\n0 67890~MOVE0~");
  }

  #[test]
  fn text_with_blank_line() {
    let mut tester = Tester::new();
    let result = tester.render("012345\r\n\r\n67890").unwrap();
    assert_eq!(result, "~MOVE0~~CLEAR_CDOWN~012345\r\n\r\n67890~MOVE0~");
    let result = tester.render("123").unwrap();
    assert_eq!(
      result,
      "~MOVE0~~CUP2~123~CLEAR_UNTIL_NEWLINE~~CDOWN1~~CLEAR_CDOWN~~CUP1~~MOVE0~"
    );
  }
}
