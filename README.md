# console_static_text

[![](https://img.shields.io/crates/v/console_static_text.svg)](https://crates.io/crates/console_static_text)

Crate for logging text that should stay in the same place in a console. This measures words to handle wrapping and has some console resizing support. Example use might be for displaying progress bars or rendering selections.

Example use with the [console](https://crates.io/crates/console) crate:

```rs
use console_static_text::ConsoleSize;
use console_static_text::ConsoleStaticText;

let mut static_text = ConsoleStaticText::new(|| {
  let size = console::Term::stderr().size();
  ConsoleSize {
    rows: Some(size.0),
    cols: Some(size.1),
  }
});

static_text.eprint("initial\ntext");
// will clear the previous text and put this new text
static_text.eprint("new text");

// or get and output the text manually
if let Some(text) = static_text.render("new text") {
  eprint!("{}", text);
}

// clear out the previous text
static_text.eprint_clear();

// todo: document hanging indent support
```
