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
