#[derive(PartialEq, Debug)]
pub enum WordToken<'a> {
  Word(&'a str),
  WhiteSpace(char),
  LfNewLine,
  CrlfNewLine,
}

impl<'a> WordToken<'a> {
  pub fn len(&self) -> usize {
    match self {
      WordToken::Word(text) => text.len(),
      WordToken::WhiteSpace(c) => c.len_utf8(),
      WordToken::LfNewLine => 1,
      WordToken::CrlfNewLine => 2,
    }
  }
}

/// Takes a string and tokenizes it into words, whitespace, and newlines.
pub fn tokenize_words(text: &str) -> impl Iterator<Item = WordToken> {
  TokenIterator {
    text,
    current_index: 0,
  }
}

struct TokenIterator<'a> {
  text: &'a str,
  current_index: usize,
}

impl<'a> Iterator for TokenIterator<'a> {
  type Item = WordToken<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    let remaining_text = &self.text[self.current_index..];
    if remaining_text.is_empty() {
      return None; // end of string
    }

    let whitespace_or_newline_index =
      find_whitespace_or_newline(&remaining_text);
    let token = if whitespace_or_newline_index == Some(0) {
      let c = remaining_text.chars().next().unwrap();
      match c {
        '\n' => WordToken::LfNewLine,
        // guaranteed by find_whitespace_or_newline to be \r\n
        '\r' => WordToken::CrlfNewLine,
        _ => WordToken::WhiteSpace(c),
      }
    } else {
      let word_end_index =
        whitespace_or_newline_index.unwrap_or(remaining_text.len());
      let next = &remaining_text[..word_end_index];
      WordToken::Word(next)
    };
    self.current_index += token.len();
    Some(token)
  }
}

fn find_whitespace_or_newline(text: &str) -> Option<usize> {
  let mut chars = text.char_indices().peekable();
  while let Some((index, c)) = chars.next() {
    match c {
      '\n' => return Some(index),
      '\r' if chars.peek().map(|(_, c)| *c) == Some('\n') => {
        return Some(index)
      }
      '\r' => {} // don't bother with \r only newlines... skip
      c if c.is_whitespace() => return Some(index),
      _ => {}
    }
  }
  None
}

#[cfg(test)]
mod tokenize_tests {
  use super::*;

  #[test]
  fn tokenize_words_2_words() {
    let result = tokenize_words("hello world");
    assert_eq!(
      result.collect::<Vec<_>>(),
      [
        WordToken::Word("hello"),
        WordToken::WhiteSpace(' '),
        WordToken::Word("world")
      ]
    );
  }

  #[test]
  fn tokenize_words_newline() {
    let result = tokenize_words("hello\nworld\r\n\n\r\n\rtest");
    assert_eq!(
      result.collect::<Vec<_>>(),
      [
        WordToken::Word("hello"),
        WordToken::LfNewLine,
        WordToken::Word("world"),
        WordToken::CrlfNewLine,
        WordToken::LfNewLine,
        WordToken::CrlfNewLine,
        WordToken::Word("\rtest")
      ]
    );
  }

  #[test]
  fn tokenize_words_newline_spaces() {
    let result = tokenize_words("hello \n  world");
    assert_eq!(
      result.collect::<Vec<_>>(),
      [
        WordToken::Word("hello"),
        WordToken::WhiteSpace(' '),
        WordToken::LfNewLine,
        WordToken::WhiteSpace(' '),
        WordToken::WhiteSpace(' '),
        WordToken::Word("world")
      ]
    );
  }

  #[test]
  fn tokenize_words_tab_char() {
    let result = tokenize_words("hello\tworld");
    assert_eq!(
      result.collect::<Vec<_>>(),
      [
        WordToken::Word("hello"),
        WordToken::WhiteSpace('\t'),
        WordToken::Word("world")
      ]
    );
  }

  #[test]
  fn tokenize_words_single_word() {
    let result = tokenize_words("hello");
    assert_eq!(result.collect::<Vec<_>>(), [WordToken::Word("hello"),]);
  }

  #[test]
  fn tokenize_words_leading_trailing_whitespace() {
    let result = tokenize_words(" hello ");
    assert_eq!(
      result.collect::<Vec<_>>(),
      [
        WordToken::WhiteSpace(' '),
        WordToken::Word("hello"),
        WordToken::WhiteSpace(' ')
      ]
    );
  }

  #[test]
  fn tokenize_words_with_rune_character() {
    let result = tokenize_words("hello⌘ ⌘world");
    assert_eq!(
      result.collect::<Vec<_>>(),
      [
        WordToken::Word("hello⌘"),
        WordToken::WhiteSpace(' '),
        WordToken::Word("⌘world")
      ]
    );
  }

  #[test]
  fn tokenize_words_single_rune_character() {
    let result = tokenize_words("⌘");
    assert_eq!(result.collect::<Vec<_>>(), [WordToken::Word("⌘"),]);
  }
}
