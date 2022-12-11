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

pub struct ConsoleStaticTextOptions {
  /// Function to strip ANSI codes.
  pub strip_ansi_codes: Box<dyn (Fn(&str) -> Cow<str>) + Send>,
  /// Function to get the terminal width.
  pub terminal_width: Box<dyn (Fn() -> u16) + Send>,
}

pub struct ConsoleStaticText {
  strip_ansi_codes: Box<dyn (Fn(&str) -> Cow<str>) + Send>,
  terminal_width: Box<dyn (Fn() -> u16) + Send>,
  last_lines: Vec<Line>,
  last_terminal_width: u16,
}

impl std::fmt::Debug for ConsoleStaticText {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("StaticText")
      .field("last_lines", &self.last_lines)
      .field("last_terminal_width", &self.last_terminal_width)
      .finish()
  }
}

impl ConsoleStaticText {
  pub fn new(options: ConsoleStaticTextOptions) -> Self {
    Self {
      strip_ansi_codes: options.strip_ansi_codes,
      terminal_width: options.terminal_width,
      last_lines: Vec::new(),
      last_terminal_width: 0,
    }
  }

  pub fn eprint_clear(&mut self) {
    if let Some(text) = self.get_clear_text() {
      eprint!("{}", text);
    }
  }

  pub fn get_clear_text(&mut self) -> Option<String> {
    let terminal_width = (self.terminal_width)();
    let last_lines = self.get_last_lines(terminal_width);
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
    self.get_update_text_with_width(new_text, (self.terminal_width)())
  }

  pub fn eprint_with_width(
    &mut self,
    new_text: &str,
    terminal_width: u16,
  ) {
    if let Some(text) = self.get_update_text_with_width(new_text, terminal_width) {
      eprint!("{}", text);
    }
  }

  pub fn get_update_text_with_width(
    &mut self,
    new_text: &str,
    terminal_width: u16,
  ) -> Option<String> {
    let is_terminal_width_different =
      terminal_width != self.last_terminal_width;
    let last_lines = self.get_last_lines(terminal_width);
    let new_lines =
      self.render_text_to_lines(new_text, terminal_width as usize);
    let result = if !are_collections_equal(&last_lines, &new_lines) {
      let mut text = String::new();
      text.push_str(VTS_MOVE_TO_ZERO_COL);
      if !last_lines.is_empty() {
        text.push_str(&vts_move_up(last_lines.len() - 1));
      }
      if is_terminal_width_different {
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
    self.last_terminal_width = terminal_width;
    result
  }

  fn get_last_lines(&mut self, terminal_width: u16) -> Vec<Line> {
    // render based on how the text looks right now
    let terminal_width = if self.last_terminal_width < terminal_width {
      self.last_terminal_width
    } else {
      terminal_width
    };

    if terminal_width == self.last_terminal_width {
      self.last_lines.drain(..).collect()
    } else {
      // render the last text with the current terminal width
      let line_texts = self
        .last_lines
        .drain(..)
        .map(|l| l.text)
        .collect::<Vec<_>>();
      self.render_text_to_lines(&line_texts.join("\n"), terminal_width as usize)
    }
  }

  fn render_text_to_lines(
    &self,
    text: &str,
    terminal_width: usize,
  ) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut line_width = 0;
    let mut current_whitespace = String::new();
    for token in self.tokenize_words(text) {
      match token {
        WordToken::Word(word, word_width) => {
          let is_word_longer_than_line = word_width > terminal_width;
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
                line_width = 0;
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
              line_width = 0;
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
