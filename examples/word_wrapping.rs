use console_static_text::ConsoleStaticText;
use std::time::Duration;

fn main() {
  let mut static_text = ConsoleStaticText::new_sized().unwrap();

  let text = format!(
    "{}\nPress ctrl+c to exit...",
    "some words repeated ".repeat(40).trim(),
  );
  let mut last_size = None;

  loop {
    let mut delay_ms = 60;
    let current_size = static_text.console_size();

    if last_size.is_some() && last_size.unwrap() != current_size {
      // debounce while the user is resizing
      delay_ms = 200;
    } else {
      // this will not update the console when the size hasn't
      // changed since the output should be the same
      static_text.eprint_with_size(&text, current_size);
    }

    std::thread::sleep(Duration::from_millis(delay_ms));
    last_size = Some(current_size);
  }
}
