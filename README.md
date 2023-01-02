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
std::thread::sleep_ms(1000);

// will clear the previous text and put this new text
static_text.eprint("new text");
std::thread::sleep_ms(1000);

// or get and output the text manually
if let Some(text) = static_text.render("new text") {
  eprint!("{}", text);
  std::thread::sleep_ms(1000);
}

// clear out the previous text
static_text.eprint_clear();
```

## Hanging indentation

To get hanging indentation, you can use the lower level "items" api.

```rs
static_text.eprint_items(vec![
  TextItem::Text("Some non-hanging text."),
  TextItem::HangingText {
    text: "some long text that will wrap at a certain width",
    indent: 4,
  },
].into_iter());
```

This is useful when implementing something like a selection UI where you want text to wrap with hanging indentation.

## "sized" feature

By default, this crate encourages you to use your own functionality for getting the console size since you'll likely already have a dependency that does that, but if not, then you can use the `sized` Cargo.toml feature.

```toml
[dependencies]
console_static_text = { version = "...", features = ["sized"] }
```

Then you can use the `new_sized` function, which will get the console size automatically:

```rs
let mut static_text = ConsoleStaticText::new_sized();

static_text.eprint("initial\ntext");
std::thread::sleep_ms(1000);

static_text.eprint("next text");
```
