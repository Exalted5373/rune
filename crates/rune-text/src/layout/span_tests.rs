use super::*;

#[test]
fn flattening_keeps_a_spans_family_and_features_and_later_spans_win() {
    let hebrew = Family::Named("Noto Serif Hebrew");
    let greek = Family::Named("Gentium Plus");
    let spans = [Span::new(0, 10).family(hebrew).features("'ss01'"), Span::new(4, 6).family(greek), Span::new(5, 8).bold()];
    assert_eq!(
        flatten_spans(&spans),
        [
            Span::new(0, 4).family(hebrew).features("'ss01'"),
            Span::new(4, 5).family(greek).features("'ss01'"),
            Span::new(5, 6).family(greek).features("'ss01'").bold(),
            Span::new(6, 8).family(hebrew).features("'ss01'").bold(),
            Span::new(8, 10).family(hebrew).features("'ss01'"),
        ]
    );
    assert_eq!(
        flatten_spans(&[Span::new(0, 4).family(hebrew), Span::new(2, 6).family(hebrew)]),
        [Span::new(0, 6).family(hebrew)],
        "neighbours in one family merge"
    );
}

#[test]
fn a_spans_family_and_features_key_its_shaping() {
    let style = TextStyle::new(13.0, 400.0);
    let key = |spans: &[Span]| text_hash("שלום עולם", spans, style, 2.0, None);
    let plain = key(&[Span::new(0, 8)]);
    let family = key(&[Span::new(0, 8).family(Family::Named("Noto Serif Hebrew"))]);
    let features = key(&[Span::new(0, 8).features("'ss01'")]);
    assert!(plain != family && plain != features && family != features);
}
