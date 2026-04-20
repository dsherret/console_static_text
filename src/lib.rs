use std::borrow::Cow;
use std::io::Write;

use ansi::strip_ansi_codes;
use unicode_width::UnicodeWidthStr;
use word::WordToken;
use word::tokenize_words;

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

// A line described as a sequence of borrowed segments from the source text
// plus a hanging-indent prefix. We defer allocating the final `String` until
// we know the line will actually be displayed — items above the console
// height, or paragraphs above a tall item's visible window, never pay the
// concatenation cost.
struct PendingLine<'a> {
  indent: usize,
  segments: Vec<&'a str>,
  total_bytes: usize,
  char_width: usize,
}

impl<'a> PendingLine<'a> {
  fn new(indent: usize) -> Self {
    Self {
      indent,
      segments: Vec::new(),
      total_bytes: 0,
      char_width: indent,
    }
  }

  fn push_segment(&mut self, s: &'a str, visible_width: usize) {
    self.segments.push(s);
    self.total_bytes += s.len();
    self.char_width += visible_width;
  }

  fn into_line(self) -> Line {
    let mut text = String::with_capacity(self.indent + self.total_bytes);
    for _ in 0..self.indent {
      text.push(' ');
    }
    for seg in self.segments {
      text.push_str(seg);
    }
    Line {
      char_width: self.char_width,
      text,
    }
  }

  fn has_content(&self) -> bool {
    !self.segments.is_empty() || self.indent > 0
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
          cols: size.map(|s| s.0.0),
          rows: size.map(|s| s.1.0),
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

  pub fn eprint_clear(&mut self) -> std::io::Result<()> {
    if let Some(text) = self.render_clear() {
      std::io::stderr().write_all(text.as_bytes())?;
    }
    Ok(())
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

  pub fn eprint(&mut self, new_text: &str) -> std::io::Result<()> {
    if let Some(text) = self.render(new_text) {
      std::io::stderr().write_all(text.as_bytes())?;
    }
    Ok(())
  }

  pub fn eprint_with_size(
    &mut self,
    new_text: &str,
    size: ConsoleSize,
  ) -> std::io::Result<()> {
    if let Some(text) = self.render_with_size(new_text, size) {
      std::io::stderr().write_all(text.as_bytes())?;
    }
    Ok(())
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
    text_items: impl DoubleEndedIterator<Item = &'a TextItem<'a>>,
  ) -> std::io::Result<()> {
    self.eprint_items_with_size(text_items, self.console_size())
  }

  pub fn eprint_items_with_size<'a>(
    &mut self,
    text_items: impl DoubleEndedIterator<Item = &'a TextItem<'a>>,
    size: ConsoleSize,
  ) -> std::io::Result<()> {
    if let Some(text) = self.render_items_with_size(text_items, size) {
      std::io::stderr().write_all(text.as_bytes())?;
    }
    Ok(())
  }

  pub fn render_items<'a>(
    &mut self,
    text_items: impl DoubleEndedIterator<Item = &'a TextItem<'a>>,
  ) -> Option<String> {
    self.render_items_with_size(text_items, self.console_size())
  }

  pub fn render_items_with_size<'a>(
    &mut self,
    text_items: impl DoubleEndedIterator<Item = &'a TextItem<'a>>,
    size: ConsoleSize,
  ) -> Option<String> {
    let is_terminal_different_size = size != self.last_size;
    let last_lines = self.get_last_lines(size);
    let new_lines = render_items(text_items, size);
    // new_lines are already wrapped to the terminal width and truncated
    // to the height by render_items, so we only need to ANSI-strip the
    // text to mirror what raw_render_last_items would produce.
    let last_lines_for_new_lines: Vec<Line> = new_lines
      .iter()
      .map(|line| Line {
        char_width: line.char_width,
        text: strip_ansi_codes(&line.text).into_owned(),
      })
      .collect();
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
          if !is_terminal_different_size
            && let Some(last_line) = last_lines.get(i)
            && last_line.char_width > new_line.char_width
          {
            text.push_str(VTS_CLEAR_UNTIL_NEWLINE);
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
      std::mem::take(&mut self.last_lines)
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
  text_items: impl DoubleEndedIterator<Item = &'a TextItem<'a>>,
  size: ConsoleSize,
) -> Vec<Line> {
  let terminal_width = size.cols.map(|c| c as usize);
  let terminal_height = size.rows.map(|c| c as usize);

  // process items bottom-up so thousands of text items don't force
  // rendering work we'd immediately truncate away. accumulate lines
  // in reverse, stopping once we've filled the console height.
  let mut iter = text_items.rev().peekable();
  let mut rev_lines: Vec<Line> = match terminal_height {
    Some(h) if iter.peek().is_some() => Vec::with_capacity(h),
    _ => Vec::new(),
  };
  'outer: for item in iter {
    if let Some(h) = terminal_height
      && rev_lines.len() >= h
    {
      break;
    }
    let (text, indent) = match item {
      TextItem::Text(text) => (text.as_ref(), 0usize),
      TextItem::HangingText { text, indent } => {
        (text.as_ref(), *indent as usize)
      }
    };
    let remaining = terminal_height.map(|h| h - rev_lines.len());
    let pending =
      render_text_to_pending(text, indent, terminal_width, remaining);
    for pl in pending.into_iter().rev() {
      rev_lines.push(pl.into_line());
      if let Some(h) = terminal_height
        && rev_lines.len() >= h
      {
        break 'outer;
      }
    }
  }
  rev_lines.reverse();
  let lines = rev_lines;

  // ensure there's always 1 line
  if lines.is_empty() {
    vec![Line::new(String::new())]
  } else {
    lines
  }
}

fn truncate_lines_height(mut lines: Vec<Line>, size: ConsoleSize) -> Vec<Line> {
  if let Some(terminal_height) = size.rows.map(|c| c as usize)
    && lines.len() > terminal_height
  {
    let cutoff_index = lines.len() - terminal_height;
    lines.drain(..cutoff_index);
  }
  lines
}

// Produces pending lines for a single text item, processing paragraphs
// (newline-delimited segments) bottom-up and stopping once `max_lines` is
// satisfied. This means early paragraphs of a tall item are never word-wrapped
// if they'd be truncated away anyway.
fn render_text_to_pending<'a>(
  text: &'a str,
  hanging_indent: usize,
  terminal_width: Option<usize>,
  max_lines: Option<usize>,
) -> Vec<PendingLine<'a>> {
  if text.is_empty() || max_lines == Some(0) {
    return Vec::new();
  }

  let paragraphs: Vec<&'a str> = text.split_terminator('\n').collect();
  // paragraph i was preceded by `\n` in the source when i > 0; when it's a
  // middle/early paragraph the trailing `\r` belongs to a CRLF pair and must
  // be trimmed so it doesn't leak into the rendered line
  let paragraph_at = |i: usize| -> &'a str {
    let p = paragraphs[i];
    if i + 1 < paragraphs.len() {
      p.strip_suffix('\r').unwrap_or(p)
    } else {
      p
    }
  };

  match terminal_width {
    None => {
      // no wrapping — each paragraph is exactly one line
      let start =
        max_lines.map_or(0, |max| paragraphs.len().saturating_sub(max));
      (start..paragraphs.len())
        .map(|i| {
          let p = paragraph_at(i);
          let width = UnicodeWidthStr::width(strip_ansi_codes(p).as_ref());
          let mut pl = PendingLine::new(0);
          if !p.is_empty() {
            pl.push_segment(p, width);
          }
          pl
        })
        .collect()
    }
    Some(terminal_width) => {
      let mut result: Vec<PendingLine<'a>> = Vec::new();
      'outer: for i in (0..paragraphs.len()).rev() {
        let p = paragraph_at(i);
        let mut paragraph_lines =
          wrap_paragraph(p, hanging_indent, terminal_width);
        if paragraph_lines.is_empty() {
          // an empty or whitespace-only paragraph still occupies one line
          paragraph_lines.push(PendingLine::new(0));
        }
        for pl in paragraph_lines.into_iter().rev() {
          result.push(pl);
          if let Some(max) = max_lines
            && result.len() >= max
          {
            break 'outer;
          }
        }
      }
      result.reverse();
      result
    }
  }
}

fn wrap_paragraph<'a>(
  text: &'a str,
  hanging_indent: usize,
  terminal_width: usize,
) -> Vec<PendingLine<'a>> {
  let mut lines: Vec<PendingLine<'a>> = Vec::new();
  let mut current_line = PendingLine::new(0);
  let mut line_width: usize = 0;
  let mut pending_whitespace: Option<&'a str> = None;

  for token in tokenize_words(text) {
    match token {
      WordToken::Word(word) => {
        let word_width =
          UnicodeWidthStr::width(strip_ansi_codes(word).as_ref());
        let is_word_longer_than_half_line =
          hanging_indent + word_width > (terminal_width / 2);
        if is_word_longer_than_half_line {
          // flush pending whitespace if it still fits on the line
          if let Some(ws) = pending_whitespace.take()
            && line_width < terminal_width
          {
            let ws_width = visible_whitespace_width(ws);
            current_line.push_segment(ws, ws_width);
          }
          // break the word at char boundaries across multiple lines,
          // preserving ANSI escapes as zero-width segments
          for ansi_token in ansi::tokenize(word) {
            let chunk = &word[ansi_token.range.clone()];
            if ansi_token.is_escape {
              current_line.push_segment(chunk, 0);
              continue;
            }
            let mut seg_start = 0;
            let mut seg_width = 0;
            let mut byte_pos = 0;
            for c in chunk.chars() {
              let c_len = c.len_utf8();
              if let Some(char_width) =
                unicode_width::UnicodeWidthChar::width(c)
              {
                if line_width + char_width > terminal_width {
                  if byte_pos > seg_start {
                    current_line
                      .push_segment(&chunk[seg_start..byte_pos], seg_width);
                  }
                  lines.push(std::mem::replace(
                    &mut current_line,
                    PendingLine::new(hanging_indent),
                  ));
                  line_width = hanging_indent;
                  seg_start = byte_pos;
                  seg_width = 0;
                }
                line_width += char_width;
                seg_width += char_width;
              }
              byte_pos += c_len;
            }
            if byte_pos > seg_start {
              current_line.push_segment(&chunk[seg_start..byte_pos], seg_width);
            }
          }
        } else {
          if line_width + word_width > terminal_width {
            lines.push(std::mem::replace(
              &mut current_line,
              PendingLine::new(hanging_indent),
            ));
            line_width = hanging_indent;
            pending_whitespace = None;
          }
          if let Some(ws) = pending_whitespace.take() {
            let ws_width = visible_whitespace_width(ws);
            current_line.push_segment(ws, ws_width);
          }
          current_line.push_segment(word, word_width);
          line_width += word_width;
        }
      }
      WordToken::WhiteSpace(ws) => {
        pending_whitespace = Some(ws);
        line_width += visible_whitespace_width(ws);
      }
      WordToken::LfNewLine | WordToken::CrlfNewLine => {
        // the caller splits on '\n' before invoking this function, so
        // paragraphs never contain newline tokens
        debug_assert!(false, "unexpected newline token in wrap_paragraph");
      }
    }
  }

  if current_line.has_content() {
    lines.push(current_line);
  }
  lines
}

fn visible_whitespace_width(s: &str) -> usize {
  s.chars()
    .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(1))
    .sum()
}

fn are_collections_equal<T: PartialEq>(a: &[T], b: &[T]) -> bool {
  a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a == b)
}

#[cfg(test)]
mod test {
  use std::sync::Arc;
  use std::sync::Mutex;

  use crate::ConsoleSize;
  use crate::ConsoleStaticText;
  use crate::TextItem;
  use crate::VTS_CLEAR_CURSOR_DOWN;
  use crate::VTS_CLEAR_UNTIL_NEWLINE;
  use crate::VTS_MOVE_TO_ZERO_COL;
  use crate::vts_move_down;
  use crate::vts_move_up;

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
          ConsoleStaticText::new(move || *size.lock().unwrap())
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

  // Lots of text items must only render the bottom ones that fit on
  // screen — see https://github.com/dsherret/console_static_text/issues/1
  #[test]
  fn truncates_many_items_to_console_height() {
    let size = ConsoleSize {
      cols: Some(20),
      rows: Some(3),
    };
    let mut s = ConsoleStaticText::new(move || size);
    let items: Vec<TextItem> = (0..1000)
      .map(|i| TextItem::new_owned(format!("line {}", i)))
      .collect();
    let result = s.render_items_with_size(items.iter(), size).unwrap();
    assert!(result.contains("line 997"));
    assert!(result.contains("line 998"));
    assert!(result.contains("line 999"));
    assert!(!result.contains("line 996"));
    assert!(!result.contains("line 0"));
  }

  // A single item with thousands of paragraphs should only render the ones
  // that fit — the paragraph-level bottom-up pass skips wrapping for the rest.
  #[test]
  fn truncates_many_paragraphs_in_single_item() {
    let size = ConsoleSize {
      cols: Some(20),
      rows: Some(3),
    };
    let mut s = ConsoleStaticText::new(move || size);
    let text = (0..1000)
      .map(|i| format!("line {}", i))
      .collect::<Vec<_>>()
      .join("\n");
    let items = [TextItem::new_owned(text)];
    let result = s.render_items_with_size(items.iter(), size).unwrap();
    assert!(result.contains("line 997"));
    assert!(result.contains("line 998"));
    assert!(result.contains("line 999"));
    assert!(!result.contains("line 996"));
    assert!(!result.contains("line 0"));
  }
}
