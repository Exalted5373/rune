//! `Popover::on_close_tx` and `Select::on_toggle_tx` run in the press's change with its `Tx`:
//! the state they set and the wake they ask that `Tx` for land together, with no poll between.
use std::time::Duration;

use rune::{Cx, El, Env, Runtime, Tx, el};
use rune_kit::{Kit, Popover, Select, root};

#[derive(Default)]
struct State { open: bool }

/// Flips `open` and asks the change's `Tx` for a wake an hour out, which nothing else asks for.
fn flip(s: &mut State, tx: &mut Tx) {
    s.open = !s.open;
    tx.wake_after(Duration::from_secs(3600));
}

fn popover(s: &State, _: &mut Cx<State>) -> El {
    let k = Kit::neutral(true);
    let mut layer = Vec::new();
    let p = Popover::new("details").trigger(el().size(40.0, 20.0)).open(s.open)
        .child(el().size(100.0, 40.0)).on_close_tx(flip);
    root(el().w_full().h_full().child(p.build(&k, &mut layer)), layer)
}

fn select(s: &State, _: &mut Cx<State>) -> El {
    let k = Kit::neutral(true);
    let mut layer = Vec::new();
    let sel = Select::new("choice", &["First", "Second"]).width(120.0).open(s.open).on_toggle_tx(flip);
    root(el().w_full().h_full().child(sel.build(&k, &mut layer)), layer)
}

/// Settles `rt`, then presses at the point `at` gives and lands that change; the press must
/// flip `open`.
fn press(mut rt: Runtime<State>, at: impl Fn(&Runtime<State>) -> (f32, f32)) -> Runtime<State> {
    for _ in 0..60 { rt.frame(1.0 / 60.0); }
    assert_eq!(rt.wake_deadline(), None, "settled, nothing asks for a wake");
    let open = rt.state.open;
    let (x, y) = at(&rt);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    assert_ne!(rt.state.open, open, "the press flips it");
    rt
}

fn woken_an_hour_out(rt: &Runtime<State>) -> bool {
    rt.wake_deadline().is_some_and(|at| at > 3000.0)
}

#[test]
fn popover_on_close_tx_closes_with_the_press_and_its_tx() {
    let rt = press(Runtime::new(State { open: true }, popover, (400.0, 400.0), Env::new(true, 1.0)), |_| (380.0, 380.0));
    assert!(!rt.state.open);
    assert!(woken_an_hour_out(&rt), "the press's Tx took the wake: {:?}", rt.wake_deadline());
}

#[test]
fn select_on_toggle_tx_opens_with_the_press_and_its_tx() {
    let rt = press(Runtime::new(State { open: false }, select, (400.0, 400.0), Env::new(true, 1.0)), |rt| {
        let ([x, y, w, h], _) = rt.presented("choice-trigger").expect("the trigger");
        (x + w / 2.0, y + h / 2.0)
    });
    assert!(rt.state.open);
    assert!(woken_an_hour_out(&rt), "the press's Tx took the wake: {:?}", rt.wake_deadline());
}
