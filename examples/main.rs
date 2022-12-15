use std::time::Duration;

use console_static_text::ConsoleSize;
use console_static_text::ConsoleStaticText;

pub fn main() {
  let mut static_text = ConsoleStaticText::new(|| console_size());

  let mut count = 0;
  let mut last_size = None;
  loop {
    // The size is requested here so it can be used to inform
    // the width of the progress bars. That same size is then used
    // for rendering. If you have no need for the console size then
    // you can just pass a string to `static_text.eprint(...)`
    let size = static_text.console_size();
    let mut sleep_ms = 120;

    if last_size.is_some() && size != last_size.unwrap() {
      // debounce when resizing
      sleep_ms = 200;
    } else {
      let mut text = format!("{}\n\n", count);
      text.push_str(concat!(
        "Some example text that will span multiple ",
        "lines when the terminal width is small enough.\n"
      ));
      text.push_str(&render_progress_bar(count % 100, 100, size.cols.unwrap()));
      static_text.eprint_with_size(&text, size);
    }

    count += 1;
    last_size = Some(size);
    std::thread::sleep(Duration::from_millis(sleep_ms));
  }
}

fn console_size() -> ConsoleSize {
  let size = console::Term::stderr().size();
  ConsoleSize {
    rows: Some(size.0),
    cols: Some(size.1),
  }
}

fn render_progress_bar(
  current_bytes: usize,
  total_bytes: usize,
  terminal_width: u16,
) -> String {
  let mut text = String::new();
  let max_width =
    std::cmp::max(10, std::cmp::min(75, terminal_width as i32 - 5)) as usize;
  let total_bars = max_width - 2; // open and close brace
  let percent_done = current_bytes as f64 / total_bytes as f64;
  let completed_bars = (total_bars as f64 * percent_done).floor() as usize;
  text.push_str("[");
  if completed_bars != total_bars {
    if completed_bars > 0 {
      text.push_str(&format!("{}{}", "#".repeat(completed_bars - 1), ">"));
    }
    text.push_str(&"-".repeat(total_bars - completed_bars))
  } else {
    text.push_str(&"#".repeat(completed_bars));
  }
  text.push(']');

  text
}
