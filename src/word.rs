#[derive(PartialEq, Debug)]
pub enum WordToken<'a> {
  Word(&'a str),
  WhiteSpace(char),
  NewLine,
}

pub fn tokenize_words(text: &str) -> Vec<WordToken> {
  let mut start_index = 0;
  let mut tokens = Vec::new();
  for (index, c) in text.char_indices() {
    if c.is_whitespace() || c == '\n' {
      let new_word_text = &text[start_index..index];
      if !new_word_text.is_empty() {
        tokens.push(WordToken::Word(new_word_text));
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
    tokens.push(WordToken::Word(new_word_text));
  }
  tokens
}

#[cfg(test)]
mod tokenize_tests {
  use super::*;

  #[test]
  fn tokenize_words_2_words() {
    let result = tokenize_words("hello world");
    assert_eq!(
      result,
      vec![
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
      result,
      vec![
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
      result,
      vec![
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
      result,
      vec![
        WordToken::Word("hello"),
        WordToken::WhiteSpace('\t'),
        WordToken::Word("world")
      ]
    );
  }

  #[test]
  fn tokenize_words_single_word() {
    let result = tokenize_words("hello");
    assert_eq!(result, vec![WordToken::Word("hello"),]);
  }

  #[test]
  fn tokenize_words_leading_trailing_whitespace() {
    let result = tokenize_words(" hello ");
    assert_eq!(
      result,
      vec![
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
      result,
      vec![
        WordToken::Word("hello⌘"),
        WordToken::WhiteSpace(' '),
        WordToken::Word("⌘world")
      ]
    );
  }

  #[test]
  fn tokenize_words_single_rune_character() {
    let result = tokenize_words("⌘");
    assert_eq!(result, vec![WordToken::Word("⌘"),]);
  }
}
