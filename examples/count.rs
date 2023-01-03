use console_static_text::ConsoleStaticText;
use std::time::Duration;

fn main() {
  let mut static_text = ConsoleStaticText::new_sized().unwrap();

  for i in 0..200 {
    static_text.eprint(&i.to_string());
    std::thread::sleep(Duration::from_millis(30));
  }

  static_text.eprint_clear();
}
