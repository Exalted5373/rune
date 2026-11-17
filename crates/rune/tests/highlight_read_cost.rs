//! Cost fixture for keyed highlight reads in one long text: an editor that marks every word
//! (spelling, grammar, headings) and reads a few of those marks back with `Cx::window_layout`.
//!
//! `cargo test -p rune --profile perf --test highlight_read_cost -- --ignored --nocapture`
//! prints the milliseconds to settle after a commit with 500 and 2,000 highlights. A read lays out
//! the highlights its key names, so the time grows with the reads and the text, not with
//! reads times highlights.
use std::cell::RefCell;
use std::time::Instant;

use rune::{Color, Cx, El, Env, Runtime, TextHighlight, TextStyle, col, text};

struct S {
    words: usize,
    reads: usize,
    /// Bumped to make the next frame lay the text out again.
    round: usize,
    seen: RefCell<usize>,
}

fn view(s: &S, cx: &mut Cx<S>) -> El {
    let found = (0..s.reads).filter(|r| cx.window_layout(&format!("w{}", r * s.words / s.reads)).is_some()).count();
    *s.seen.borrow_mut() = found;
    let body: String = (0..s.words).map(|i| format!("word{i} ")).collect();
    let mut at = 0;
    let mut t = text(format!("{body}{}", s.round)).font(TextStyle::new(14.0, 400.0).lh(20.0));
    for i in 0..s.words {
        let len = format!("word{i}").len();
        t = t.text_highlight(TextHighlight::new(at..at + len, Color::WHITE).key(format!("w{i}")));
        at += len + 1;
    }
    col().w_full().child(t)
}

fn settle(rt: &mut Runtime<S>) {
    for _ in 0..600 { if !rt.frame(1.0 / 60.0) { return; } }
    panic!("never settled");
}

/// Milliseconds to settle after a commit, median of five, with `words` keyed highlights.
fn per_commit(words: usize, reads: usize) -> f64 {
    let s = S { words, reads, round: 0, seen: RefCell::new(0) };
    let mut rt = Runtime::new(s, view, (900.0, 4000.0), Env::new(true, 1.0));
    rt.frame(0.0);
    settle(&mut rt);
    assert_eq!(*rt.state.seen.borrow(), reads, "every read found its highlight");
    let mut times: Vec<f64> = (0..5).map(|_| {
        rt.update(|s, _| s.round += 1);
        let start = Instant::now();
        settle(&mut rt);
        start.elapsed().as_secs_f64() * 1e3
    }).collect();
    times.sort_by(f64::total_cmp);
    times[2]
}

#[test]
#[ignore = "manual cost fixture: run with --profile perf --ignored --nocapture"]
fn keyed_highlight_reads_cost_what_they_read() {
    for words in [500, 2_000] {
        println!("{words} highlights, 20 keyed reads: {:.1} ms per commit", per_commit(words, 20));
    }
}
