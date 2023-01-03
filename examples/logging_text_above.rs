use console_static_text::ConsoleStaticText;
use std::io::Write;
use std::time::Duration;

fn main() {
  let mut static_text = ConsoleStaticText::new_sized().unwrap();

  for i in 0..200 {
    if i % 10 == 0 {
      let size = static_text.console_size();
      let mut new_text = String::new();

      // first clear the existing static text
      if let Some(text) = static_text.render_clear_with_size(size) {
        new_text.push_str(&text);
      }

      // log the new text
      new_text.push_str(&format!("Hello from {}\n", i));

      // then redraw the static text
      if let Some(text) = static_text.render_with_size(&i.to_string(), size) {
        new_text.push_str(&text);
      }

      // now output everything in one go
      std::io::stderr().write_all(new_text.as_bytes()).unwrap();
    } else {
      static_text.eprint(&i.to_string());
    }

    std::thread::sleep(Duration::from_millis(30));
  }

  static_text.eprint_clear();
}
