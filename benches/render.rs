use console_static_text::ConsoleSize;
use console_static_text::ConsoleStaticText;
use console_static_text::TextItem;

fn main() {
  divan::main();
}

const COLS: u16 = 80;
const ROWS: u16 = 25;

const SIZE: ConsoleSize = ConsoleSize {
  cols: Some(COLS),
  rows: Some(ROWS),
};

// Render thousands of small text items into a 25-row console — the bottom-up
// item walk should keep work proportional to the visible window, not N.
#[divan::bench(args = [25, 100, 1_000, 10_000])]
fn many_items(bencher: divan::Bencher, n: usize) {
  let items: Vec<TextItem<'static>> = (0..n)
    .map(|i| TextItem::new_owned(format!("item line {}", i)))
    .collect();
  bencher.bench_local(|| {
    let mut s = ConsoleStaticText::new(|| SIZE);
    s.render_items_with_size(divan::black_box(items.iter()), SIZE)
  });
}

// Single text item with many newline-separated paragraphs — the
// paragraph-level bottom-up pass should skip wrapping for invisible ones.
#[divan::bench(args = [25, 100, 1_000, 10_000])]
fn single_item_many_paragraphs(bencher: divan::Bencher, n: usize) {
  let text = (0..n)
    .map(|i| format!("paragraph {}", i))
    .collect::<Vec<_>>()
    .join("\n");
  let items = [TextItem::new_owned(text)];
  bencher.bench_local(|| {
    let mut s = ConsoleStaticText::new(|| SIZE);
    s.render_items_with_size(divan::black_box(items.iter()), SIZE)
  });
}

// A single item containing one very long unbroken word. Exercises the
// long-word per-char break path; only the bottom 25 wrapped lines should
// survive, but every char above must still be examined to know where to
// break.
#[divan::bench(args = [80, 800, 8_000, 80_000])]
fn single_long_line(bencher: divan::Bencher, len: usize) {
  let text: String = std::iter::repeat('x').take(len).collect();
  let items = [TextItem::new_owned(text)];
  bencher.bench_local(|| {
    let mut s = ConsoleStaticText::new(|| SIZE);
    s.render_items_with_size(divan::black_box(items.iter()), SIZE)
  });
}

// Items that need word wrapping — exercises the per-word path that previously
// allocated a `Vec<AnsiToken>` for every word.
#[divan::bench(args = [25, 100, 1_000])]
fn wrapped_items(bencher: divan::Bencher, n: usize) {
  let items: Vec<TextItem<'static>> = (0..n)
    .map(|i| {
      TextItem::new_owned(format!(
        "this is a moderately long line {} that needs wrapping at eighty columns to exercise the word wrapping path",
        i
      ))
    })
    .collect();
  bencher.bench_local(|| {
    let mut s = ConsoleStaticText::new(|| SIZE);
    s.render_items_with_size(divan::black_box(items.iter()), SIZE)
  });
}
