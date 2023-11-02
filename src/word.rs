#[derive(PartialEq, Debug)]
pub enum WordToken<'a> {
  Word(&'a str),
  WhiteSpace(char),
  NewLine,
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
      None
    } else if let Some(end_word_index) =
      remaining_text.find(|c: char| c.is_whitespace() || c == '\n')
    {
      if end_word_index == 0 {
        // it's a newline or whitespace
        let c = remaining_text.chars().next().unwrap();
        self.current_index += c.len_utf8();
        Some(if c == '\n' {
          WordToken::NewLine
        } else {
          WordToken::WhiteSpace(c)
        })
      } else {
        let next = &remaining_text[..end_word_index];
        self.current_index += next.len();
        Some(WordToken::Word(next))
      }
    } else {
      self.current_index += remaining_text.len();
      Some(WordToken::Word(remaining_text))
    }
  }
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
    let result = tokenize_words("hello\nworld");
    assert_eq!(
      result.collect::<Vec<_>>(),
      [
        WordToken::Word("hello"),
        WordToken::NewLine,
        WordToken::Word("world")
      ]
    );
  }

  #[test]
  fn tokenize_words_newline_spaces() {
    let result = tokenize_words("hello \n world");
    assert_eq!(
      result.collect::<Vec<_>>(),
      [
        WordToken::Word("hello"),
        WordToken::WhiteSpace(' '),
        WordToken::NewLine,
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
