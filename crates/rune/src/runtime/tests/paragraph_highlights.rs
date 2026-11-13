//! Keyed highlights and links in a field laid out by paragraph (`TextLayout::Paragraphs`): each
//! is found, and drawn, where its own paragraph lays its bytes out.
use super::*;
use crate::text::{Link, TextHighlight};
use crate::{Color, TextEdit, TextStyle, text_input};

const TEXT: &str = "first mark line\nsecond line with the later mark\nthird line end";
const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
const SCALE: f32 = 2.0;
const LINE: f32 = 20.0;

struct State {
    edit: TextEdit,
}

/// Content bytes of the first paragraph's `mark` and the second's.
fn marks() -> ((usize, usize), (usize, usize)) {
    let first = TEXT.find("mark").unwrap();
    let later = TEXT.rfind("mark").unwrap();
    ((first, first + 4), (later, later + 4))
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let ((a0, b0), (a1, b1)) = marks();
    let field = text_input(&s.edit, TextStyle::new(14.0, 400.0).lh(LINE))
        .key("field")
        .size(280.0, 100.0)
        .on_edit(|s: &mut State| &mut s.edit)
        .text_highlight(TextHighlight::new(a0..b0, RED).key("first-mark"))
        .text_highlight(TextHighlight::new(a1..b1, BLUE).key("later-mark"))
        .link(Link::new(a1..b1, "later-link"));
    col().w_full().h_full().padding(8.0).child(field)
}

fn runtime() -> Runtime<State> {
    let mut rt = Runtime::new(State { edit: TextEdit::new(TEXT) }, view, (320.0, 200.0), Env::new(true, SCALE));
    rt.frame(0.0);
    let n = rt.nodes.iter().find(|n| n.key.as_deref() == Some("field")).unwrap();
    assert!(n.text_layout.as_ref().unwrap().single().is_none(), "the field lays out by paragraph");
    rt
}

/// The field's text top-left in points, and the centre (points) of content bytes `a..b`'s box,
/// from the field's own paragraph-aware layout.
fn expected(rt: &Runtime<State>, a: usize, b: usize) -> ([f32; 2], (f32, f32)) {
    let n = rt.nodes.iter().find(|n| n.key.as_deref() == Some("field")).unwrap();
    let top_left = [n.target[0] + n.inset[0], n.target[1] + n.inset[1]];
    let boxes = n.text_layout.as_ref().unwrap().view().range_boxes(a, b, false);
    assert_eq!(boxes.len(), 1, "one line box");
    let bb = boxes[0].0;
    let at = (top_left[0] + (bb.x0 + bb.x1) as f32 * 0.5 / SCALE, top_left[1] + (bb.y0 + bb.y1) as f32 * 0.5 / SCALE);
    (top_left, at)
}

fn near(found: (f32, f32), want: (f32, f32)) -> bool {
    (found.0 - want.0).abs() < 0.5 && (found.1 - want.1).abs() < 0.5
}

#[test]
fn find_reaches_highlights_and_links_in_every_paragraph_of_a_field() {
    let rt = runtime();
    let ((a0, b0), (a1, b1)) = marks();
    let (top_left, first) = expected(&rt, a0, b0);
    let (_, later) = expected(&rt, a1, b1);
    assert!(first.1 > top_left[1] && first.1 < top_left[1] + LINE, "the first mark is on the first line: {first:?}");
    assert!(later.1 > top_left[1] + LINE && later.1 < top_left[1] + 2.0 * LINE, "the later mark is on the second line: {later:?}");

    let found = rt.find("first-mark").expect("the first paragraph's highlight");
    assert!(near(found, first), "first-mark at {found:?}, laid out at {first:?}");
    let found = rt.find("later-mark").expect("the second paragraph's highlight");
    assert!(near(found, later), "later-mark at {found:?}, laid out at {later:?}");
    let found = rt.find("later-link").expect("the second paragraph's link");
    assert!(near(found, later), "later-link at {found:?}, laid out at {later:?}");
}

#[test]
fn a_field_paints_each_highlight_in_its_own_paragraph_once() {
    let rt = runtime();
    let ((a0, b0), (a1, b1)) = marks();
    let (_, first) = expected(&rt, a0, b0);
    let (_, later) = expected(&rt, a1, b1);
    // Each highlight's fill, centred on its bytes' box across and on its line down.
    let painted = |fill: [f32; 4]| -> Vec<(f32, f32)> {
        rt.instances.iter().filter(|i| i.fill == fill)
            .map(|i| ((i.rect[0] + i.rect[2] * 0.5) / SCALE, (i.rect[1] + i.rect[3] * 0.5) / SCALE))
            .collect()
    };
    let red = painted([1.0, 0.0, 0.0, 1.0]);
    assert_eq!(red.len(), 1, "the first mark draws once, in its own paragraph: {red:?}");
    assert!((red[0].0 - first.0).abs() < 0.5 && (red[0].1 - first.1).abs() < LINE * 0.5, "first mark drawn at {:?}, laid out at {first:?}", red[0]);
    let blue = painted([0.0, 0.0, 1.0, 1.0]);
    assert_eq!(blue.len(), 1, "the later mark draws once, in its own paragraph: {blue:?}");
    assert!((blue[0].0 - later.0).abs() < 0.5 && (blue[0].1 - later.1).abs() < LINE * 0.5, "later mark drawn at {:?}, laid out at {later:?}", blue[0]);
}
