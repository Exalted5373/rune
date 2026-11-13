//! Scholarly text: Genesis 1:1 as an interlinear that wraps right to left
//! (`El::dir(Direction::Rtl)`, each English gloss in its own direction), Hebrew falling back
//! to a bundled font (`text::set_script_fonts`), a Greek span in its own family
//! (`Span::family`), a right-to-left paragraph opening with a Latin reference, and Arabic,
//! Syriac and Ge'ez in bundled fonts. The fonts are the subsets in `tests/fonts`, so only
//! their samples draw.
//!
//!   cargo run -p rune --example interlinear -- --snap interlinear.png
//!   cargo run -p rune --example interlinear -- --snap interlinear-narrow.png --scene narrow
use rune::tok::{self, type_};
use rune::{AppSpec, Cx, Direction, El, Family, Span, TextStyle, col, rich, row, text};

const SAMPLES: &str = include_str!("../tests/fonts/samples.txt");
const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/Inter-Regular.ttf"),
    include_bytes!("../tests/fonts/NotoSerifHebrew-Regular-subset.ttf"),
    include_bytes!("../tests/fonts/Amiri-Regular-subset.ttf"),
    include_bytes!("../tests/fonts/PolytonicTestSerif-Regular-subset.ttf"),
    include_bytes!("../tests/fonts/NotoSansSyriac-Regular-subset.ttf"),
    include_bytes!("../tests/fonts/NotoSerifEthiopic-Regular-subset.ttf"),
];
const GLOSSES: [&str; 7] = ["In the beginning", "created", "God", "(object)", "the heavens", "and (object)", "the earth."];

struct State {
    width: f32,
}

fn sample(script: &str) -> &'static str {
    SAMPLES.lines().find_map(|l| l.strip_prefix(script)?.strip_prefix('\t')).unwrap_or_default()
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let t = tok::theme(true);
    let original = TextStyle::new(22.0, 400.0);
    let words = sample("Hebr").split(' ').zip(GLOSSES).enumerate().map(|(i, (word, gloss))| {
        col().key(format!("word{i}")).items_center().gap(2.0)
            .child(text(word).font(original).color(t.text))
            .child(text(gloss).font(type_::BODY).color(t.text_secondary).dir(Direction::Auto))
    });
    let interlinear = row().key("interlinear").w(s.width).dir(Direction::Rtl).flex_wrap().gaps(10.0, 14.0).children(words);
    let greek = sample("Grek");
    let greek_line = format!("John 1:1 {greek}");
    let at = greek_line.len() - greek.len();
    let span = Span::new(at, greek_line.len()).family(Family::Named("Polytonic Test Serif"));
    let reference = text(format!("Gen 1:1 {}", sample("Hebr"))).font(original).color(t.text).w(s.width).wrap().dir(Direction::Rtl);
    let line = |script: &str, family: &'static str| text(sample(script)).font(original.family(Family::Named(family)).lh(40.0)).color(t.text).w(s.width).wrap();
    col().w_full().h_full().padding(24.0).gap(18.0).fill(t.bg)
        .child(interlinear)
        .child(rich(greek_line, vec![span]).font(original).color(t.text).w(s.width))
        .child(reference)
        .child(line("Arab", "Amiri"))
        .child(line("Syrc", "Noto Sans Syriac"))
        .child(line("Ethi", "Noto Serif Ethiopic"))
}

fn main() {
    rune::text::set_script_fonts(&[("Hebr", "Noto Serif Hebrew")]);
    rune::run(AppSpec::<State> {
        scenes: &[("narrow", |s| s.width = 300.0)],
        fonts: FONTS,
        families: ("Inter", "Inter"),
        ..AppSpec::new("Interlinear", (720.0, 560.0), || State { width: 640.0 }, view)
    });
}
