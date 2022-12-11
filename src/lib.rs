use std::borrow::Cow;

const VTS_MOVE_TO_ZERO_COL: &str = "\x1B[0G";
const VTS_CLEAR_CURRENT_LINE: &str = "\x1B[2K";
const VTS_CLEAR_CURSOR_DOWN: &str = "\x1B[J";
const VTS_CLEAR_UNTIL_NEWLINE: &str = "\x1B[K";

fn vts_move_up(count: usize) -> String {
  if count == 0 {
    String::new()
  } else {
    format!("\x1B[{}A", count)
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

enum WordToken<'a> {
  Word(&'a str, usize),
  WhiteSpace(char),
  NewLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleSize {
  pub cols: u16,
  pub rows: u16,
}

pub struct ConsoleStaticTextOptions {
  /// Function to strip ANSI codes.
  pub strip_ansi_codes: Box<dyn (Fn(&str) -> Cow<str>) + Send>,
  /// Function to get the terminal width.
  pub console_size: Box<dyn (Fn() -> ConsoleSize) + Send>,
}

pub struct ConsoleStaticText {
  strip_ansi_codes: Box<dyn (Fn(&str) -> Cow<str>) + Send>,
  console_size: Box<dyn (Fn() -> ConsoleSize) + Send>,
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
  pub fn new(options: ConsoleStaticTextOptions) -> Self {
    Self {
      strip_ansi_codes: options.strip_ansi_codes,
      console_size: options.console_size,
      last_lines: Vec::new(),
      last_size: ConsoleSize { cols: 0, rows: 0 },
    }
  }

  pub fn eprint_clear(&mut self) {
    if let Some(text) = self.get_clear_text() {
      eprint!("{}", text);
    }
  }

  pub fn get_clear_text(&mut self) -> Option<String> {
    let size = (self.console_size)();
    let last_lines = self.get_last_lines(size);
    if !last_lines.is_empty() {
      Some(format!(
        "{}{}{}",
        VTS_MOVE_TO_ZERO_COL,
        vts_move_up(last_lines.len()),
        VTS_CLEAR_CURSOR_DOWN
      ))
    } else {
      None
    }
  }

  pub fn eprint(&mut self, new_text: &str) {
    if let Some(text) = self.get_update_text(new_text) {
      eprint!("{}", text);
    }
  }

  pub fn get_update_text(&mut self, new_text: &str) -> Option<String> {
    self.get_update_text_with_size(new_text, (self.console_size)())
  }

  pub fn eprint_with_size(&mut self, new_text: &str, size: ConsoleSize) {
    if let Some(text) = self.get_update_text_with_size(new_text, size) {
      eprint!("{}", text);
    }
  }

  pub fn get_update_text_with_size(
    &mut self,
    new_text: &str,
    size: ConsoleSize,
  ) -> Option<String> {
    self.get_update_text_with_size_from_items(
      vec![TextItem::Text(new_text)].into_iter(),
      size,
    )
  }

  pub fn get_update_text_with_size_from_items<'a>(
    &mut self,
    text_items: impl Iterator<Item = TextItem<'a>>,
    size: ConsoleSize,
  ) -> Option<String> {
    let is_terminal_different = size != self.last_size;
    let last_lines = self.get_last_lines(size);
    let new_lines = self.render_items(text_items, size);
    let result = if !are_collections_equal(&last_lines, &new_lines) {
      let mut text = String::new();
      text.push_str(VTS_MOVE_TO_ZERO_COL);
      if !last_lines.is_empty() {
        text.push_str(&vts_move_up(last_lines.len() - 1));
      }
      if is_terminal_different {
        text.push_str(VTS_CLEAR_CURSOR_DOWN);
      }
      for i in 0..std::cmp::max(last_lines.len(), new_lines.len()) {
        let last_line = last_lines.get(i);
        let new_line = new_lines.get(i);
        if i > 0 {
          text.push('\n');
        }
        if let Some(new_line) = new_line {
          text.push_str(&new_line.text);
          if let Some(last_line) = last_line {
            if last_line.char_width > new_line.char_width {
              text.push_str(VTS_CLEAR_UNTIL_NEWLINE);
            }
          }
        } else {
          text.push_str(VTS_CLEAR_CURRENT_LINE);
        }
      }
      if last_lines.len() > new_lines.len() {
        text.push_str(&vts_move_up(last_lines.len() - new_lines.len()));
      }
      Some(text)
    } else {
      None
    };
    self.last_lines = new_lines;
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
      self.render_items(vec![TextItem::new(&text)].into_iter(), size)
    }
  }

  fn render_items<'a>(
    &self,
    text_items: impl Iterator<Item = TextItem<'a>>,
    size: ConsoleSize,
  ) -> Vec<Line> {
    let mut lines = Vec::new();
    let terminal_width = size.cols as usize;
    for item in text_items {
      match item {
        TextItem::Text(text) => {
          lines.extend(self.render_text_to_lines(text, 0, terminal_width))
        }
        TextItem::HangingText { text, indent } => {
          lines.extend(self.render_text_to_lines(
            text,
            indent as usize,
            terminal_width,
          ));
        }
      }
    }
    let terminal_height = size.rows as usize;
    if lines.len() > terminal_height {
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
    } else {
      lines
    }
  }

  fn render_text_to_lines(
    &self,
    text: &str,
    hanging_indent: usize,
    terminal_width: usize,
  ) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut line_width = 0;
    let mut current_whitespace = String::new();
    for token in self.tokenize_words(text) {
      match token {
        WordToken::Word(word, word_width) => {
          let is_word_longer_than_line =
            hanging_indent + word_width > terminal_width;
          if is_word_longer_than_line {
            // break it up onto multiple lines with indentation
            if !current_whitespace.is_empty() {
              if line_width < terminal_width {
                current_line.push_str(&current_whitespace);
              }
              current_whitespace = String::new();
            }
            for c in word.chars() {
              if line_width == terminal_width {
                lines.push(Line {
                  char_width: line_width,
                  text: current_line,
                });
                current_line = String::new();
                current_line.push_str(&" ".repeat(hanging_indent));
                line_width = hanging_indent;
              }
              current_line.push(c);
              line_width += 1;
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
    lines
  }

  fn tokenize_words<'a>(&self, text: &'a str) -> Vec<WordToken<'a>> {
    let mut start_index = 0;
    let mut tokens = Vec::new();
    for (index, c) in text.char_indices() {
      if c.is_whitespace() || c == '\n' {
        let new_word_text = &text[start_index..index];
        if !new_word_text.is_empty() {
          tokens.push(self.create_word_token(new_word_text));
        }

        if c == '\n' {
          tokens.push(WordToken::NewLine);
        } else {
          tokens.push(WordToken::WhiteSpace(c));
        }

        start_index = index + c.len_utf8(); // start at next char
      }
    }

    let new_word_text = &text[start_index..];
    if !new_word_text.is_empty() {
      tokens.push(self.create_word_token(new_word_text));
    }
    tokens
  }

  fn create_word_token<'a>(&self, text: &'a str) -> WordToken<'a> {
    WordToken::Word(text, (self.strip_ansi_codes)(text).chars().count())
  }
}

fn are_collections_equal<T: PartialEq>(a: &[T], b: &[T]) -> bool {
  a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a == b)
}
