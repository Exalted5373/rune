use super::*;
use crate::shape::Op;
use crate::{Color, Shape};

fn square(x: f32, y: f32, side: f32) -> [[f32; 2]; 4] {
    [[x, y], [x + side, y], [x + side, y + side], [x, y + side]]
}

fn reversed(points: [[f32; 2]; 4]) -> [[f32; 2]; 4] {
    let mut p = points;
    p.reverse();
    p
}

fn cross(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// The area its triangles cover.
fn covered(area: &Area) -> f32 {
    let (v, tris) = area.mesh();
    tris.iter().map(|([a, b, c], _)| cross(v[*a as usize], v[*b as usize], v[*c as usize]).abs() * 0.5).sum()
}

/// Whether one of its triangles holds `p` inside, not on an edge.
fn painted(area: &Area, p: [f32; 2]) -> bool {
    let (v, tris) = area.mesh();
    tris.iter().any(|t| {
        let [a, b, c] = t.0.map(|i| v[i as usize]);
        let s = [cross(a, b, p), cross(b, c, p), cross(c, a, p)];
        s.iter().all(|&d| d > 1e-3) || s.iter().all(|&d| d < -1e-3)
    })
}

/// The length of the edges it antialiases, which should be its outline and nothing else.
fn outline(area: &Area) -> f32 {
    let (v, tris) = area.mesh();
    tris.iter()
        .flat_map(|(t, mask)| (0..3).filter(move |e| mask & (1 << e) != 0).map(move |e| (t[e], t[(e + 1) % 3])))
        .map(|(a, b)| super::super::dist(v[a as usize], v[b as usize]))
        .sum()
}

#[test]
fn a_hole_is_cut_out_of_the_fill_and_a_point_in_it_misses() {
    for rule in [FillRule::EvenOdd, FillRule::NonZero] {
        // The hole winds against the outline, so both rules cut it.
        let path = Path::new().polygon(square(0.0, 0.0, 30.0)).polygon(reversed(square(10.0, 10.0, 10.0)));
        let area = Area::new(&path, rule);
        assert!((covered(&area) - 800.0).abs() < 1e-2, "{rule:?}: {}", covered(&area));
        // Points off the 45° lines, where the triangles' shared edges may run.
        assert!(!painted(&area, [15.3, 14.1]), "{rule:?}: nothing painted in the hole");
        assert!(painted(&area, [5.3, 2.1]) && painted(&area, [25.2, 15.7]));
        assert!(!area.contains([15.3, 14.1]), "{rule:?}: a point in the hole misses");
        assert!(area.contains([5.3, 2.1]) && area.contains([25.2, 15.7]) && area.contains([15.3, 5.1]));
        assert!(!area.contains([-1.0, 5.0]) && !area.contains([31.0, 15.0]));
        assert!((outline(&area) - 160.0).abs() < 1e-2, "{rule:?}: only the two rings are soft: {}", outline(&area));
        assert_eq!(area.bounds(), Some([0.0, 0.0, 30.0, 30.0]));
    }
}

#[test]
fn even_odd_and_non_zero_differ_where_contours_wind_the_same_way() {
    // An inner square wound like its outline: a hole under even-odd, filled over under non-zero.
    let nested = Path::new().polygon(square(0.0, 0.0, 30.0)).polygon(square(10.0, 10.0, 10.0));
    let even = Area::new(&nested, FillRule::EvenOdd);
    let nonzero = Area::new(&nested, FillRule::NonZero);
    assert!((covered(&even) - 800.0).abs() < 1e-2);
    assert!((covered(&nonzero) - 900.0).abs() < 1e-2);
    assert!(!even.contains([15.3, 14.1]) && !painted(&even, [15.3, 14.1]));
    assert!(nonzero.contains([15.3, 14.1]) && painted(&nonzero, [15.3, 14.1]));
    assert!((outline(&nonzero) - 120.0).abs() < 1e-2, "the inner ring isn't an edge under non-zero");

    // Two overlapping islands: the overlap is inside once under non-zero, outside under even-odd.
    let islands = Path::new().polygon(square(0.0, 0.0, 20.0)).polygon(square(10.0, 10.0, 20.0));
    let even = Area::new(&islands, FillRule::EvenOdd);
    let nonzero = Area::new(&islands, FillRule::NonZero);
    assert!((covered(&nonzero) - 700.0).abs() < 1e-2, "{}", covered(&nonzero));
    assert!((covered(&even) - 600.0).abs() < 1e-2, "{}", covered(&even));
    assert!(nonzero.contains([15.3, 14.1]) && !even.contains([15.3, 14.1]));
    assert!(even.contains([5.3, 2.1]) && even.contains([25.2, 27.7]));

    // A self-crossing star: its centre winds twice.
    let star: Vec<[f32; 2]> = (0..5)
        .map(|i| {
            let t = std::f32::consts::TAU * (i * 2) as f32 / 5.0 - std::f32::consts::FRAC_PI_2;
            [50.0 + 40.0 * t.cos(), 50.0 + 40.0 * t.sin()]
        })
        .collect();
    let star = Path::new().polygon(star);
    let (even, nonzero) = (Area::new(&star, FillRule::EvenOdd), Area::new(&star, FillRule::NonZero));
    assert!(!even.contains([50.7, 51.3]) && !painted(&even, [50.7, 51.3]));
    assert!(nonzero.contains([50.7, 51.3]) && painted(&nonzero, [50.7, 51.3]));
    assert!(covered(&nonzero) > covered(&even));
}

#[test]
fn hit_testing_agrees_with_the_painted_triangles() {
    // A wavy outline with three holes, two of them crossing each other.
    let blob: Vec<[f32; 2]> = (0..240)
        .map(|i| {
            let t = std::f32::consts::TAU * i as f32 / 240.0;
            let r = 80.0 + 12.0 * (7.0 * t).sin() + 5.0 * (19.0 * t).cos();
            [100.0 + r * t.cos(), 100.0 + r * t.sin()]
        })
        .collect();
    let path = Path::new()
        .polygon(blob)
        .polygon(square(60.0, 60.0, 30.0))
        .polygon(square(75.0, 75.0, 30.0))
        .polygon(reversed(square(120.0, 110.0, 25.0)));
    for rule in [FillRule::EvenOdd, FillRule::NonZero] {
        let area = Area::new(&path, rule);
        let mut checked = 0;
        for y in 0..100 {
            for x in 0..100 {
                let p = [x as f32 * 2.0 + 0.37, y as f32 * 2.0 + 0.61];
                // Points on an edge may go either way; everywhere else they agree.
                let (v, tris) = area.mesh();
                let near_edge = tris.iter().any(|t| {
                    let [a, b, c] = t.0.map(|i| v[i as usize]);
                    [(a, b), (b, c), (c, a)].iter().any(|&(a, b)| {
                        let d = cross(a, b, p).abs() / super::super::dist(a, b).max(1e-6);
                        d < 0.05 && p[0] >= a[0].min(b[0]) - 0.05 && p[0] <= a[0].max(b[0]) + 0.05
                    })
                });
                if !near_edge {
                    assert_eq!(area.contains(p), painted(&area, p), "{rule:?} at {p:?}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 9_000, "{checked}");
    }
}

#[test]
fn empty_and_degenerate_contours_are_left_out() {
    let empty = Area::new(&Path::new(), FillRule::NonZero);
    assert!(empty.is_empty() && empty.bounds().is_none() && !empty.contains([0.0, 0.0]));
    let degenerate = Path::new()
        .polygon([[0.0, 0.0], [5.0, 5.0]])
        .polygon([[0.0, 0.0], [f32::NAN, 1.0], [4.0, 0.0], [0.0, 0.0]])
        .polygon([[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 0.0]]);
    let area = Area::new(&degenerate, FillRule::EvenOdd);
    assert_eq!(area.contours().count(), 1, "the triangle, with its repeated start dropped");
    assert_eq!(area.contours().next().map(<[_]>::len), Some(3));
    assert!((covered(&area) - 50.0).abs() < 1e-3);
    assert!(Shape::new().fill_area(&empty, Color::WHITE).is_empty(), "an empty area draws nothing");
}

#[test]
fn an_area_is_shared_by_its_clones_and_fades_in_like_a_fill() {
    let path = Path::new().move_to([1.0, 1.0]).polygon(square(0.0, 0.0, 30.0)).polygon(square(10.0, 10.0, 10.0));
    assert_eq!(path.subpaths.len(), 2, "a lone move_to is replaced");
    let area = Area::new(&path, FillRule::EvenOdd);
    let copy = area.clone();
    assert!(Arc::ptr_eq(&area.0, &copy.0));
    assert_eq!(area, Area::new(&path, FillRule::EvenOdd), "equal contours and rule");
    assert_ne!(area, Area::new(&path, FillRule::NonZero));
    assert_eq!(format!("{area:?}"), "Area { rule: EvenOdd, contours: 2, points: 8, triangles: 8 }");

    let s = Shape::new().fill_area(&area, Color::WHITE);
    assert!(!s.overlaps(), "one area is one piece");
    let Some(Op::Area { area: drawn, color }) = s.drawn(0.25).ops.first().cloned() else { panic!("an area") };
    assert!(Arc::ptr_eq(&drawn.0, &area.0), "trimming shares the triangles");
    assert!((color.a - 0.25).abs() < 1e-6);
    assert_eq!(Shape::new().fill_path_with(&path, FillRule::EvenOdd, Color::WHITE), s);
}
