//! Scholarly scripts shape completely in bundled fonts: pointed Hebrew with cantillation,
//! vocalized Arabic, polytonic Greek, Syriac and Ge'ez (samples and fonts in `fonts/`). Also
//! the app's script fallbacks and per-span families and features those fonts serve.

use std::rc::Rc;

use parley::Layout;
use rune::text::{Ink, RichLayoutOptions, TextSystem};
use rune::{Family, Span, TextDirection, TextStyle};
use swash::scale::ScaleContext;

const INTER: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
const SAMPLES: &str = include_str!("fonts/samples.txt");

/// Each sample's script, its font, the family that font registers, and whether it runs right
/// to left.
const FONTS: [(&str, &[u8], &str, bool); 5] = [
    ("Hebr", include_bytes!("fonts/NotoSerifHebrew-Regular-subset.ttf"), "Noto Serif Hebrew", true),
    ("Arab", include_bytes!("fonts/Amiri-Regular-subset.ttf"), "Amiri", true),
    ("Grek", include_bytes!("fonts/PolytonicTestSerif-Regular-subset.ttf"), "Polytonic Test Serif", false),
    ("Syrc", include_bytes!("fonts/NotoSansSyriac-Regular-subset.ttf"), "Noto Sans Syriac", true),
    ("Ethi", include_bytes!("fonts/NotoSerifEthiopic-Regular-subset.ttf"), "Noto Serif Ethiopic", false),
];

fn sample(script: &str) -> &'static str {
    SAMPLES.lines().find_map(|l| l.strip_prefix(script)?.strip_prefix('\t')).unwrap_or_else(|| panic!("no {script} sample"))
}

fn system() -> TextSystem {
    let fonts: Vec<&'static [u8]> = std::iter::once(INTER).chain(FONTS.iter().map(|f| f.1)).collect();
    TextSystem::new(&fonts, ("Inter", "Inter"))
}

fn is_mark(ch: char) -> bool {
    // The combining blocks the samples use: Greek's combining diacritics, Hebrew points and
    // accents, Arabic harakat and the superscript alef, Syriac vowels and dots.
    matches!(ch as u32, 0x0300..=0x036F | 0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7
        | 0x064B..=0x065F | 0x0670 | 0x0730..=0x074A)
}

/// One shaped glyph: its cluster's source bytes, font, id, advance and ink box (device px,
/// x right and y up from the baseline) where it's drawn.
struct Placed {
    bytes: std::ops::Range<usize>,
    font: *const u8,
    id: u32,
    advance: f32,
    ink: Option<[f32; 4]>,
}

/// Every glyph of `layout` in visual order, and each run's direction with its clusters'
/// starts in visual order.
fn placed(layout: &Layout<Ink>, scale_cx: &mut ScaleContext) -> (Vec<Placed>, Vec<(bool, Vec<usize>)>) {
    let (mut glyphs, mut runs) = (Vec::new(), Vec::new());
    for line in layout.lines() {
        let mut pen = line.metrics().offset;
        for run in line.runs() {
            let font = run.font();
            let face = swash::FontRef::from_index(font.data.as_ref(), font.index as usize).expect("a font swash reads");
            let mut scaler = scale_cx.builder(face).size(run.font_size()).build();
            runs.push((run.is_rtl(), run.visual_clusters().map(|c| c.text_range().start).collect()));
            for cluster in run.visual_clusters() {
                for g in cluster.glyphs() {
                    let ink = scaler.scale_outline(g.id as u16).map(|o| o.bounds()).filter(|b| b.max.x > b.min.x).map(|b| {
                        [pen + g.x + b.min.x, -g.y + b.min.y, pen + g.x + b.max.x, -g.y + b.max.y]
                    });
                    glyphs.push(Placed { bytes: cluster.text_range(), font: font.data.as_ref().as_ptr(), id: g.id, advance: g.advance, ink });
                    pen += g.advance;
                }
            }
        }
    }
    (glyphs, runs)
}

/// Every mark of `text` has a zero-advance glyph of its own drawn over or under its base: the
/// mark's ink is centred within the base's ink (give or take a quarter of the base's advance).
fn assert_marks_attach(script: &str, text: &str, glyphs: &[Placed]) {
    let mut clusters: Vec<std::ops::Range<usize>> = glyphs.iter().map(|g| g.bytes.clone()).collect();
    clusters.dedup();
    let mut marked = 0;
    for bytes in clusters {
        let marks = text[bytes.clone()].chars().filter(|&c| is_mark(c)).count();
        if marks == 0 {
            continue;
        }
        marked += 1;
        let cluster: Vec<&Placed> = glyphs.iter().filter(|g| g.bytes == bytes).collect();
        let what = format!("{script} {:?}", &text[bytes.clone()]);
        let bases: Vec<&&Placed> = cluster.iter().filter(|g| g.advance > 0.0).collect();
        assert_eq!(bases.len(), 1, "{what}: one spacing base glyph");
        let base = bases[0];
        let base_ink = base.ink.expect("the base draws");
        let slack = base.advance * 0.25;
        let mark_glyphs: Vec<&&Placed> = cluster.iter().filter(|g| g.advance == 0.0).collect();
        assert!(!mark_glyphs.is_empty(), "{what}: its marks draw as glyphs of their own");
        for m in mark_glyphs {
            let ink = m.ink.unwrap_or_else(|| panic!("{what}: mark glyph {} has no ink", m.id));
            let centre = (ink[0] + ink[2]) / 2.0;
            assert!(
                base_ink[0] - slack <= centre && centre <= base_ink[2] + slack,
                "{what}: mark {} centred at {centre:.1}, off its base's ink {:.1}..{:.1}", m.id, base_ink[0], base_ink[2]
            );
            assert!(ink != base_ink, "{what}: mark {} drawn where its base is", m.id);
        }
    }
    let expected = if matches!(script, "Ethi") { 0 } else { 1 };
    assert!(marked >= expected, "{script}: the sample has marked letters");
}

#[test]
fn pointed_and_vocalized_scripts_shape_with_every_glyph_and_their_marks_attached() {
    let mut ts = system();
    let mut scale_cx = ScaleContext::new();
    for (script, font, family, rtl) in FONTS {
        let text = sample(script);
        let layout = ts.layout_rich(text, &[], TextStyle::new(24.0, 400.0).family(Family::Named(family)), 2.0, None);
        assert_eq!(layout.len(), 1, "{script}: one line");
        let (glyphs, runs) = placed(&layout, &mut scale_cx);
        assert!(glyphs.iter().all(|g| g.id != 0), "{script}: no missing glyphs");
        assert!(glyphs.iter().all(|g| g.font == font.as_ptr()), "{script}: every glyph from {family}");
        for (run_rtl, starts) in &runs {
            assert_eq!(*run_rtl, rtl, "{script}: run direction");
            let ordered = if rtl { starts.windows(2).all(|w| w[0] > w[1]) } else { starts.windows(2).all(|w| w[0] < w[1]) };
            assert!(ordered, "{script}: clusters in visual order run {}", if rtl { "right to left" } else { "left to right" });
        }
        assert_marks_attach(script, text, &glyphs);
    }
}

/// Glyphs of letters in `text`'s layout.
fn letters(ts: &mut TextSystem, text: &str, style: TextStyle) -> Vec<Placed> {
    let layout = ts.layout_rich(text, &[], style, 2.0, None);
    let glyphs = placed(&layout, &mut ScaleContext::new()).0;
    assert!(glyphs.iter().all(|g| g.id != 0), "no missing glyphs in {text}");
    let letters: Vec<Placed> = glyphs.into_iter().filter(|g| text[g.bytes.clone()].chars().any(char::is_alphabetic)).collect();
    assert!(!letters.is_empty());
    letters
}

#[test]
fn scripts_fall_back_to_the_apps_fonts_before_the_systems() {
    let mut ts = system();
    let style = TextStyle::new(16.0, 400.0);
    let hebrew = FONTS[0].1.as_ptr();
    // Without a map, Hebrew takes the system's fonts (or draws nothing) whatever the app bundles.
    let unmapped = ts.layout_rich(sample("Hebr"), &[], style, 2.0, None);
    assert!(placed(&unmapped, &mut ScaleContext::new()).0.iter().all(|g| g.font != hebrew), "an unmapped script never picks a bundled font");
    let map = [("Hebr", "Noto Serif Hebrew"), ("arab", "Amiri"), ("Grek", "Polytonic Test Serif"), ("Syrc", "Missing Family, Noto Sans Syriac"), ("Ethi", "Noto Serif Ethiopic"), ("Xxxx", "Inter")];
    assert!(ts.set_script_fonts(&map));
    assert!(!ts.set_script_fonts(&map), "the same map changes nothing");
    for (script, font, _, _) in FONTS {
        // Inter draws Greek, and the element's own family comes first.
        let want = if script == "Grek" { INTER } else { font };
        let glyphs = letters(&mut ts, sample(script), style);
        assert!(glyphs.iter().all(|g| g.font == want.as_ptr()), "{script}: letters in Inter's text take the mapped font");
    }
    // A sans without Greek (the Ethiopic face) leaves Greek to the map, ahead of the system's
    // symbol fonts, some of which draw Greek.
    let fonts: Vec<&'static [u8]> = FONTS.iter().map(|f| f.1).collect();
    let mut ethiopic = TextSystem::new(&fonts, ("Noto Serif Ethiopic", "Noto Serif Ethiopic"));
    ethiopic.set_script_fonts(&map);
    let glyphs = letters(&mut ethiopic, sample("Grek"), style);
    assert!(glyphs.iter().all(|g| g.font == FONTS[2].1.as_ptr()), "Grek: Greek falls back to the mapped font");
    assert!(ts.set_script_fonts(&[]));
    let cleared = ts.layout_rich(sample("Hebr"), &[], style, 2.0, None);
    assert!(placed(&cleared, &mut ScaleContext::new()).0.iter().all(|g| g.font != hebrew), "clearing the map drops it");
}

#[test]
fn a_span_shapes_in_its_own_family() {
    let mut ts = system();
    let mut scale_cx = ScaleContext::new();
    let text = format!("John 1:1 {} and Genesis 1:1 {}", "Ἐν ἀρχῇ ἦν ὁ λόγος", "בְּרֵאשִׁ֖ית בָּרָ֣א");
    let greek = text.find('Ἐ').unwrap()..text.find(" and").unwrap();
    let hebrew = text.find('ב').unwrap()..text.len();
    let spans = [
        Span::new(greek.start, greek.end).family(Family::Named("Polytonic Test Serif")),
        Span::new(hebrew.start, hebrew.end).family(Family::Named("Noto Serif Hebrew")),
    ];
    let style = TextStyle::new(16.0, 400.0);
    let layout = ts.layout_rich(&text, &spans, style, 2.0, None);
    let glyphs = placed(&layout, &mut scale_cx).0;
    assert!(glyphs.iter().all(|g| g.id != 0));
    for g in &glyphs {
        let want = if greek.contains(&g.bytes.start) {
            FONTS[2].1
        } else if hebrew.contains(&g.bytes.start) {
            FONTS[0].1
        } else {
            INTER
        };
        assert!(g.font == want.as_ptr(), "{:?} from the wrong font", &text[g.bytes.clone()]);
    }
    assert_marks_attach("Hebr", &text, &glyphs);
    let plain = ts.layout_rich(&text, &spans[..1], style, 2.0, None);
    assert!(!Rc::ptr_eq(&layout, &plain), "a span's family is part of the cache key");
}

/// The glyph ids each `0` of `text` draws with.
fn zeros(ts: &mut TextSystem, text: &str, spans: &[Span], style: TextStyle) -> Vec<u32> {
    let layout = ts.layout_rich(text, spans, style, 2.0, None);
    let mut scale_cx = ScaleContext::new();
    placed(&layout, &mut scale_cx).0.into_iter().filter(|g| &text[g.bytes.clone()] == "0").map(|g| g.id).collect()
}

#[test]
fn a_spans_features_add_to_the_elements_and_win_on_the_same_tag() {
    let mut ts = system();
    let style = TextStyle::new(16.0, 400.0);
    let plain = zeros(&mut ts, "0 0 0", &[], style);
    assert_eq!(plain[0], plain[1]);
    let slashed = zeros(&mut ts, "0 0 0", &[Span::new(2, 3).features("'zero'")], style);
    assert_eq!((slashed[0], slashed[2]), (plain[0], plain[0]), "outside the span, plain zeros");
    assert_ne!(slashed[1], plain[1], "inside it, Inter's slashed zero");
    let element = style.features("'zero'");
    let undone = zeros(&mut ts, "0 0 0", &[Span::new(2, 3).features("'zero' 0")], element);
    assert_eq!((undone[0], undone[2]), (slashed[1], slashed[1]), "the element's feature holds outside the span");
    assert_eq!(undone[1], plain[1], "the span's setting of the same tag wins");
    let kept = zeros(&mut ts, "0 0 0", &[Span::new(2, 3).features("'ss01'")], element);
    assert_eq!(kept[1], slashed[1], "a span's other features keep the element's");
}

#[test]
fn an_explicit_direction_orders_runs_and_starts_lines_from_its_side() {
    let mut ts = system();
    let style = TextStyle::new(16.0, 400.0);
    let text = format!("Gen 1:1 {}", sample("Hebr"));
    let runs = |ts: &mut TextSystem, direction| {
        let layout = ts.layout_rich_with(&text, &[], style, 2.0, Some(2000.0), RichLayoutOptions::new().direction(direction));
        let line = layout.lines().next().unwrap();
        let first = line.runs().next().unwrap();
        (layout.is_rtl(), text[first.text_range()].starts_with("Gen"), line.metrics().offset)
    };
    let (rtl, latin_first, offset) = runs(&mut ts, TextDirection::Auto);
    assert!(!rtl && latin_first && offset == 0.0, "the Latin reference makes the paragraph left to right");
    let (rtl, latin_first, offset) = runs(&mut ts, TextDirection::Rtl);
    assert!(rtl && !latin_first && offset > 0.0, "right to left, the Hebrew leads from the right edge");
    let hebrew_first = format!("{} Gen 1:1", sample("Hebr"));
    let layout = ts.layout_rich_with(&hebrew_first, &[], style, 2.0, None, RichLayoutOptions::new().direction(TextDirection::Ltr));
    assert!(!layout.is_rtl(), "an explicit Ltr overrides the first strong letter");
}
