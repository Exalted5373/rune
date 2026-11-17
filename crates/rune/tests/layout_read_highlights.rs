//! Layout reads of text highlights and keyed elements stay right while a list scrolls (frames
//! that reuse what a lookup found before) and after a commit moves what a key names.
use std::cell::RefCell;

use rune::{Color, Cx, El, Env, Runtime, ScrollSpec, TextHighlight, TextStyle, col, text};

struct S {
    hit_row: usize,
    /// Element keys read besides the highlight, so lookups go through an index.
    extra_reads: usize,
    seen: RefCell<Option<[f32; 4]>>,
    seen_rows: RefCell<Vec<Option<[f32; 4]>>>,
}

fn view(s: &S, cx: &mut Cx<S>) -> El {
    *s.seen.borrow_mut() = cx.window_layout(&format!("row{}/mark", s.hit_row));
    *s.seen_rows.borrow_mut() = (0..s.extra_reads).map(|i| cx.window_layout(&format!("row{i}"))).collect();
    let rows = (0..30).fold(col().w_full(), |c, i| {
        // Every row's text has a highlight keyed `mark`: the scoped read picks one row's.
        let body = text(format!("row {i} text with a mark in it"))
            .font(TextStyle::new(14.0, 400.0).lh(20.0))
            .text_highlight(TextHighlight::new(18..22, Color::WHITE).key("mark"));
        c.child(col().key(format!("row{i}")).w_full().h(40.0).py(10.0).child(body))
    });
    col().w_full().h_full().child(col().key("list").w_full().h(200.0).scroll(ScrollSpec { stick_bottom: false }).child(rows))
}

fn settle(rt: &mut Runtime<S>) {
    for _ in 0..600 { if !rt.frame(1.0 / 60.0) { return; } }
    panic!("never settled");
}

/// The highlight `mark` in row `row` as `[x, y, w, h]`, and row `i`s rect, from the tree.
fn mark_of(rt: &Runtime<S>, row: usize) -> [f32; 4] {
    let scope = format!("/row{row}/");
    let e = rt.elements().into_iter()
        .find(|e| e.key.as_deref() == Some("mark") && e.path.contains(&scope))
        .unwrap_or_else(|| panic!("mark in row {row}"));
    e.rect
}

fn row_of(rt: &Runtime<S>, i: usize) -> [f32; 4] {
    let key = format!("row{i}");
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key.as_str())).expect("row").rect
}

fn close(a: [f32; 4], b: [f32; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.75)
}

fn check(rt: &Runtime<S>, when: &str) {
    let seen = rt.state.seen.borrow().unwrap_or_else(|| panic!("{when}: read saw nothing"));
    let mark = mark_of(rt, rt.state.hit_row);
    assert!(close(seen, mark), "{when}: read {seen:?}, mark at {mark:?}");
    for (i, seen) in rt.state.seen_rows.borrow().iter().enumerate() {
        let seen = seen.unwrap_or_else(|| panic!("{when}: row{i} read saw nothing"));
        let row = row_of(rt, i);
        assert!(close(seen, row), "{when}: row{i} read {seen:?}, row at {row:?}");
    }
}

fn reads_follow_scrolls_and_commits(extra_reads: usize) {
    let s = S { hit_row: 6, extra_reads, seen: RefCell::new(None), seen_rows: RefCell::new(Vec::new()) };
    let mut rt = Runtime::new(s, view, (300.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    settle(&mut rt);
    check(&rt, "first layout");

    for dy in [-120.0, -160.0, 200.0] {
        let before = mark_of(&rt, rt.state.hit_row)[1];
        rt.wheel(100.0, 100.0, dy);
        settle(&mut rt);
        assert_ne!(mark_of(&rt, rt.state.hit_row)[1], before, "the wheel scrolled");
        check(&rt, &format!("wheeled {dy}"));
    }

    // A commit names another row's highlight; frames after it look it up in the new tree.
    rt.update(|s, _| s.hit_row = 2);
    settle(&mut rt);
    check(&rt, "after the commit");
    rt.wheel(100.0, 100.0, 400.0);
    settle(&mut rt);
    check(&rt, "wheeled back after the commit");
}

#[test]
fn a_scoped_highlight_read_follows_scrolls_and_commits() {
    reads_follow_scrolls_and_commits(0);
}

#[test]
fn many_reads_follow_scrolls_and_commits() {
    reads_follow_scrolls_and_commits(12);
}

struct Marks {
    reads: Vec<String>,
    seen: RefCell<Vec<Option<[f32; 4]>>>,
}

/// One wrapped text with a keyed highlight per word; `both` names two of them, far apart.
fn marks_view(s: &Marks, cx: &mut Cx<Marks>) -> El {
    *s.seen.borrow_mut() = s.reads.iter().map(|key| cx.window_layout(key)).collect();
    let words: Vec<String> = (0..60).map(|i| format!("word{i}")).collect();
    let mut at = 0;
    let mut t = text(words.join(" ")).font(TextStyle::new(14.0, 400.0).lh(20.0)).wrap();
    for (i, word) in words.iter().enumerate() {
        let key = if i == 3 || i == 50 { "both".to_owned() } else { format!("w{i}") };
        t = t.text_highlight(TextHighlight::new(at..at + word.len(), Color::WHITE).key(key));
        at += word.len() + 1;
    }
    col().w(220.0).child(t)
}

/// A read of one key among many highlights of a text answers that key's highlights alone,
/// the union of both when two share it; otherwise a caret or menu anchored to one word lands
/// on the text's first mark or on the whole paragraph.
#[test]
fn a_read_answers_its_own_highlights_among_many_in_one_text() {
    let reads: Vec<String> = ["w0", "w7", "both", "w59"].map(String::from).to_vec();
    let s = Marks { reads: reads.clone(), seen: RefCell::new(Vec::new()) };
    let mut rt = Runtime::new(s, marks_view, (300.0, 600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    for _ in 0..600 { if !rt.frame(1.0 / 60.0) { break; } }
    let elements = rt.elements();
    let mark = |key: &str| -> Vec<[f32; 4]> {
        elements.iter().filter(|e| e.key.as_deref() == Some(key)).map(|e| e.rect).collect()
    };
    let seen = rt.state.seen.borrow().clone();
    for (key, seen) in reads.iter().zip(seen) {
        let seen = seen.unwrap_or_else(|| panic!("{key}: read saw nothing"));
        let rects = mark(key);
        assert!(!rects.is_empty(), "{key}: drawn as a mark");
        let x0 = rects.iter().map(|r| r[0]).fold(f32::INFINITY, f32::min);
        let y0 = rects.iter().map(|r| r[1]).fold(f32::INFINITY, f32::min);
        let x1 = rects.iter().map(|r| r[0] + r[2]).fold(f32::NEG_INFINITY, f32::max);
        let y1 = rects.iter().map(|r| r[1] + r[3]).fold(f32::NEG_INFINITY, f32::max);
        assert!(close(seen, [x0, y0, x1 - x0, y1 - y0]), "{key}: read {seen:?}, marks at {rects:?}");
    }
    let both = mark("both");
    assert_eq!(both.len(), 2, "two highlights share `both`");
    assert!((both[1][1] - both[0][1]).abs() > 1.0, "on different lines, so the union is not one word: {both:?}");
}
