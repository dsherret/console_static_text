# console_static_text

Zero dependency tool for updating static text in a console that measures words to handle wrapping and has some console resizing support. For an example, this could be used for displaying progress bars or inputs.

Example use with the [console](https://crates.io/crates/console) crate:

```rs
use console_static_text::StaticText;
use console_static_text::StaticTextOptions;

let term = console::Term::stderr();
let mut static_text = StaticText::new(StaticTextOptions {
  strip_ansi_codes: Box::new(console::strip_ansi_codes),
  terminal_width: Box::new(|| term.size().1),
});

if let Some(text) = static_text.update("initial\ntext") {
  eprint!("{}", text);
}

if let Some(text) = static_text.update("new text") {
  // will clear out the previous text and put this new text
  eprint!("{}", text);
}

if let Some(text) = static_text.clear() {
  // writes the escape sequences necessary
  // to clear the previous text
  eprint!{"{}", text);
}
```

Extracted out from [dprint](https://github.com/dprint/dprint) for reuse in [Deno](https://github.com/denoland/deno).
