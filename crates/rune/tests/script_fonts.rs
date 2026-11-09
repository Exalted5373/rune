//! `rune::text::set_script_fonts` reaches every text system, built before or after it, and
//! `measure`. Alone in its binary: the map is process-wide.

use rune::TextStyle;
use rune::text::TextSystem;

const INTER: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
const HEBREW: &[u8] = include_bytes!("fonts/NotoSerifHebrew-Regular-subset.ttf");
const SAMPLES: &str = include_str!("fonts/samples.txt");

/// The fonts that draw the Hebrew letters of `text`.
fn fonts(ts: &mut TextSystem, text: &str) -> Vec<*const u8> {
    let layout = ts.layout_rich(text, &[], TextStyle::new(16.0, 400.0), 2.0, None);
    let mut out = Vec::new();
    for line in layout.lines() {
        for run in line.runs() {
            if text[run.text_range()].chars().any(|c| ('\u{05D0}'..='\u{05EA}').contains(&c)) {
                out.push(run.font().data.as_ref().as_ptr());
            }
        }
    }
    assert!(!out.is_empty());
    out
}

#[test]
fn every_text_system_and_measure_follow_the_apps_script_fonts() {
    let text = SAMPLES.lines().find_map(|l| l.strip_prefix("Hebr\t")).unwrap();
    let style = TextStyle::new(16.0, 400.0);
    let mut built = TextSystem::new(&[INTER, HEBREW], ("Inter", "Inter"));
    assert!(fonts(&mut built, text).iter().all(|&f| f != HEBREW.as_ptr()));
    let unmapped = rune::text::measure(text, &[], style, None);
    rune::text::set_script_fonts(&[("Hebr", "Noto Serif Hebrew")]);
    assert!(fonts(&mut built, text).iter().all(|&f| f == HEBREW.as_ptr()), "a text system built before the map follows it");
    let mut fresh = TextSystem::new(&[INTER, HEBREW], ("Inter", "Inter"));
    assert!(fonts(&mut fresh, text).iter().all(|&f| f == HEBREW.as_ptr()), "one built after takes it up");
    assert_ne!(rune::text::measure(text, &[], style, None), unmapped, "measure shapes in the mapped font");
    rune::text::set_script_fonts(&[]);
    assert!(fonts(&mut built, text).iter().all(|&f| f != HEBREW.as_ptr()), "an empty map clears it");
    assert_eq!(rune::text::measure(text, &[], style, None), unmapped);
}
