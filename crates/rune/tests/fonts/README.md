# Script shaping fonts

Subsets of OFL fonts for `tests/script_shaping.rs` and `tests/script_fonts.rs`: each holds only
the characters of its line in `samples.txt` (Genesis 1:1 in pointed Hebrew with cantillation, the
basmala and John 1:1 in vocalized Arabic, John 1:1 in polytonic Greek with two stacked combining
sequences, and John 1:1 in vocalized Syriac and in Ge'ez), with every OpenType layout feature, cut
with fontTools:

    pyftsubset SOURCE.ttf --text-file=LINE.txt --layout-features='*' --no-hinting

| File | Source | SHA-256 of the source | Licence |
|---|---|---|---|
| `NotoSerifHebrew-Regular-subset.ttf` | Noto Serif Hebrew Regular 2.004 | `dfd5a6aefe97a99f…` | `NotoSerifHebrew-OFL.txt` |
| `Amiri-Regular-subset.ttf` | Amiri Regular 1.000 | `b092096eb992aebe…` | `Amiri-OFL.txt` |
| `PolytonicTestSerif-Regular-subset.ttf` | Gentium Plus Regular 6.200 | `2c27e7da23ba44d1…` | `PolytonicTestSerif-OFL.txt` |
| `NotoSansSyriac-Regular-subset.ttf` | Noto Sans Syriac Regular 3.000 | `4440929bf1a47bb5…` | `NotoSansSyriac-OFL.txt` |
| `NotoSerifEthiopic-Regular-subset.ttf` | Noto Serif Ethiopic Regular 2.102 | `4d14f10b7436e762…` | `NotoSerifEthiopic-OFL.txt` |

Gentium Plus reserves the names "Gentium" and "SIL", so its subset is renamed "Polytonic Test
Serif" (name IDs 1, 3, 4, 6 and 16), as the licence requires of a modified version.
