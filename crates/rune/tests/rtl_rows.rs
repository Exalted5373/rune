//! An interlinear verse's rows under `El::dir(Direction::Rtl)`: children laid out right to left,
//! line by line, with press, Tab and arrow keys following what's drawn; and a text's own
//! `El::dir` setting its paragraph direction.
mod support;

use rune::{Cx, Direction, El, Env, Key, KeyEvent, Mods, Runtime, col, el, row, text};
use support::{centre, click, find, glyphs, tick};

const WORDS: usize = 7;

#[derive(Default)]
struct S {
    group: bool,
    pressed: Vec<usize>,
}

/// Seven 50×20 words in a 200-wide wrapping row with 10-point gaps: three to a line.
fn verse(s: &S, _: &mut Cx<S>) -> El {
    let words = (0..WORDS).map(|i| {
        el().key(format!("w{i}")).size(50.0, 20.0).focusable(true).role("button").on_press(move |s: &mut S| s.pressed.push(i))
    });
    let verse = row().key("verse").w(200.0).dir(Direction::Rtl).flex_wrap().gap(10.0).children(words);
    col().child(if s.group { verse.focus_group() } else { verse })
}

fn runtime(group: bool) -> Runtime<S> {
    let mut rt = Runtime::new(S { group, ..S::default() }, verse, (200.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn press(rt: &mut Runtime<S>, key: Key) {
    rt.key(KeyEvent { key, mods: Mods::default() });
    tick(rt);
}

fn focused(rt: &Runtime<S>) -> Option<String> {
    rt.elements().into_iter().find(|e| e.focused).and_then(|e| e.key)
}

#[test]
fn words_fill_each_line_from_the_right_and_lines_run_down() {
    let rt = runtime(false);
    for i in 0..WORDS {
        let r = find(&rt, &format!("w{i}")).rect;
        let at = [150.0 - 60.0 * (i % 3) as f32, 30.0 * (i / 3) as f32];
        assert_eq!([r[0], r[1]], at, "w{i}");
    }
    let plain = Runtime::new(S::default(), |_, _| row().w(200.0).flex_wrap().gap(10.0).child(el().key("w0").size(50.0, 20.0)), (200.0, 200.0), Env::new(true, 1.0));
    let mut plain = plain;
    plain.frame(0.0);
    assert_eq!(find(&plain, "w0").rect[0], 0.0, "a row without a direction still starts at the left");
}

#[test]
fn presses_land_on_the_word_drawn_there() {
    let mut rt = runtime(false);
    click(&mut rt, (175.0, 10.0));
    click(&mut rt, (55.0, 40.0));
    click(&mut rt, (175.0, 70.0));
    assert_eq!(rt.state.pressed, [0, 5, 6]);
    let w4 = centre(&find(&rt, "w4"));
    click(&mut rt, w4);
    assert_eq!(rt.state.pressed.last(), Some(&4));
}

#[test]
fn tab_reads_the_words_in_order_from_the_right() {
    let mut rt = runtime(false);
    let mut order = Vec::new();
    for _ in 0..4 {
        press(&mut rt, Key::Tab);
        order.push(focused(&rt).unwrap());
    }
    assert_eq!(order, ["w0", "w1", "w2", "w3"]);
    let xs: Vec<f32> = order[..3].iter().map(|k| find(&rt, k).rect[0]).collect();
    assert!(xs.windows(2).all(|w| w[0] > w[1]), "each Tab moves left along the line: {xs:?}");
}

#[test]
fn arrows_in_a_group_move_by_what_is_on_screen() {
    let mut rt = runtime(true);
    press(&mut rt, Key::Tab);
    assert_eq!(focused(&rt).as_deref(), Some("w0"));
    press(&mut rt, Key::Left);
    assert_eq!(focused(&rt).as_deref(), Some("w1"), "Left goes to the word on the left, the next one");
    press(&mut rt, Key::Left);
    press(&mut rt, Key::Left);
    assert_eq!(focused(&rt).as_deref(), Some("w3"), "past a line's left end, onto the next line");
    press(&mut rt, Key::Right);
    assert_eq!(focused(&rt).as_deref(), Some("w2"), "Right goes back");
    press(&mut rt, Key::Down);
    assert_eq!(focused(&rt).as_deref(), Some("w3"), "Down still moves forward");
}

const LINE: &str = "In the beginning God created the heaven and the earth.";

/// The paragraph in `direction`, or with none set.
fn paragraph(direction: &Option<Direction>, _: &mut Cx<Option<Direction>>) -> El {
    let line = text(LINE).key("p").w(600.0).wrap();
    row().w(600.0).child(match direction { Some(d) => line.dir(*d), None => line })
}

/// The leftmost and rightmost glyph centres of the paragraph, in points.
fn extent(direction: Option<Direction>) -> (f32, f32) {
    let mut rt = Runtime::new(direction, paragraph, (600.0, 200.0), Env::new(true, 2.0));
    rt.frame(0.0);
    let xs: Vec<f32> = glyphs(&rt, "p").into_iter().map(|g| g.0).collect();
    assert!(!xs.is_empty());
    (xs.iter().copied().fold(f32::MAX, f32::min), xs.iter().copied().fold(f32::MIN, f32::max))
}

#[test]
fn a_right_to_left_paragraph_starts_its_lines_at_the_right() {
    let (left, right) = extent(None);
    assert!(left < 10.0 && right < 590.0, "English starts at the left: {left}..{right}");
    assert_eq!(extent(Some(Direction::Ltr)), (left, right));
    assert_eq!(extent(Some(Direction::Auto)), (left, right));
    let (rtl_left, rtl_right) = extent(Some(Direction::Rtl));
    assert!(rtl_right > 590.0 && rtl_left > 10.0, "a right-to-left paragraph hugs the right: {rtl_left}..{rtl_right}");
    assert!(((rtl_right - rtl_left) - (right - left)).abs() < 8.0, "the same line, moved");
}
