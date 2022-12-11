pub enum WordToken<'a> {
  Word(&'a str),
  WhiteSpace(char),
  NewLine,
}

pub fn tokenize_words<'a>(text: &'a str) -> Vec<WordToken<'a>> {
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
