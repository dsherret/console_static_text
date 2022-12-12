use std::borrow::Cow;
use std::ops::Range;

use vte::Parser;
use vte::Perform;

pub struct AnsiToken {
  pub range: Range<usize>,
  pub is_escape: bool,
}

pub fn strip_ansi_codes(text: &str) -> Cow<str> {
  let tokens = tokenize(text);
  if tokens.is_empty() || tokens.len() == 1 && !tokens[0].is_escape {
    Cow::Borrowed(text)
  } else {
    let mut final_text = String::new();
    for token in tokens {
      if !token.is_escape {
        final_text.push_str(&text[token.range]);
      }
    }
    Cow::Owned(final_text)
  }
}

/// Tokenizes the provided text into ansi escape sequences
pub fn tokenize(text: &str) -> Vec<AnsiToken> {
  let mut parser = Parser::new();
  let mut performer = Performer {
    current_end_index: 0,
    last_handled_end_index: 0,
    last_handled_start_index: 0,
    tokens: Vec::new(),
    is_current_escape: false,
  };
  for byte in text.as_bytes() {
    performer.current_end_index += 1;
    parser.advance(&mut performer, *byte);
  }
  performer.mark_end();
  performer.tokens
}

struct Performer {
  last_handled_start_index: usize,
  last_handled_end_index: usize,
  current_end_index: usize,
  tokens: Vec<AnsiToken>,
  is_current_escape: bool,
}

impl Performer {
  pub fn mark_char(&mut self, c: char) {
    if self.is_current_escape {
      let char_start_index = self.current_end_index - c.len_utf8();
      self.last_handled_start_index = char_start_index;
      self.is_current_escape = false;
    }
    self.last_handled_end_index = self.current_end_index;
  }

  pub fn mark_escape(&mut self) {
    if !self.is_current_escape {
      self.finalize(false);
      self.is_current_escape = true;
      self.last_handled_start_index = self.last_handled_end_index;
    }
    self.last_handled_end_index = self.current_end_index;
    self.finalize(true);
    self.last_handled_start_index = self.current_end_index;
  }

  pub fn mark_end(&mut self) {
    self.last_handled_end_index = self.current_end_index;
    self.finalize(self.is_current_escape);
  }

  fn finalize(&mut self, is_escape: bool) {
    let range = self.last_handled_start_index..self.last_handled_end_index;
    if !range.is_empty() {
      self.tokens.push(AnsiToken { range, is_escape });
    }
  }
}

impl Perform for Performer {
  fn print(&mut self, c: char) {
    self.mark_char(c);
  }

  fn execute(&mut self, byte: u8) {
    match byte {
      b'\n' => self.mark_char('\n'),
      b'\r' => self.mark_char('\r'),
      b'\t' => self.mark_char('\t'),
      _ => self.mark_escape(),
    }
  }

  fn hook(
    &mut self,
    _params: &vte::Params,
    _intermediates: &[u8],
    _ignore: bool,
    _action: char,
  ) {
    self.mark_escape();
  }

  fn put(&mut self, _byte: u8) {
    self.mark_escape();
  }

  fn unhook(&mut self) {
    self.mark_escape();
  }

  fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
    self.mark_escape();
  }

  fn csi_dispatch(
    &mut self,
    _params: &vte::Params,
    _intermediates: &[u8],
    _ignore: bool,
    _action: char,
  ) {
    self.mark_escape();
  }

  fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
    self.mark_escape();
  }
}

#[cfg(test)]
mod test {
  use pretty_assertions::assert_eq;

  use super::tokenize;

  #[test]
  fn should_tokenize() {
    let output = get_output("");
    assert_eq!(output, vec![]);
    let output = get_output("this is a test");
    assert_eq!(
      output,
      vec![TestToken {
        text: "this is a test".to_string(),
        is_escape: false,
      }]
    );
    let output = get_output("\x1b[mthis is \x1B[2Ka \r\n\ttest\x1b[m\x1B[2K");
    assert_eq!(
      output,
      vec![
        TestToken {
          text: "\u{1b}[m".to_string(),
          is_escape: true,
        },
        TestToken {
          text: "this is ".to_string(),
          is_escape: false,
        },
        TestToken {
          text: "\x1B[2K".to_string(),
          is_escape: true,
        },
        TestToken {
          text: "a \r\n\ttest".to_string(),
          is_escape: false,
        },
        TestToken {
          text: "\u{1b}[m".to_string(),
          is_escape: true,
        },
        TestToken {
          text: "\x1B[2K".to_string(),
          is_escape: true,
        },
      ]
    );
  }

  #[derive(Debug, PartialEq, Eq)]
  struct TestToken {
    text: String,
    is_escape: bool,
  }

  fn get_output(text: &str) -> Vec<TestToken> {
    tokenize(text)
      .into_iter()
      .map(|t| TestToken {
        text: text[t.range].to_string(),
        is_escape: t.is_escape,
      })
      .collect()
  }
}
