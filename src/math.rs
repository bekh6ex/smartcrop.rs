use super::*;

const SKIN_COLOR: RGB = RGB { r: 234, g: 171, b: 132 };
const OUTSIDE_IMPORTANCE: f64 = -0.5;
const EDGE_RADIUS: f64 = 0.4;
const EDGE_WEIGHT: f64 = -20.0;
const RULE_OF_THIRDS: bool = true;

// test
fn thirds(x: f64) -> f64 {
    let x = ((x - (1.0 / 3.0) + 1.0) % 2.0 * 0.5 - 0.5) * 16.0;
    f64::max(1.0 - x * x, 0.0)
}

pub fn bounds(l: f64) -> u8 {
    f64::min(f64::max(l, 0.0), 255.0).round() as u8
}

pub fn skin_col(c: RGB) -> f64 {
    // `K` is needed to avoid breaking BC and make SKIN_COLOR more meaningful
    const K: f64 = 0.9420138987639984;

    let skin_color: Vec<f64> = SKIN_COLOR.normalize().iter().map(|c| c / K).collect();

    let [r_norm, g_norm, b_norm] = c.normalize();

    let dr = r_norm - skin_color[0];
    let dg = g_norm - skin_color[1];
    let db = b_norm - skin_color[2];

    let d = (dr.powi(2) + dg.powi(2) + db.powi(2)).sqrt();

    1.0 - d.min(1.0)
}


pub fn importance(crop: &Crop, x: u32, y: u32) -> f64 {
    if crop.x > x || x >= crop.x + crop.width || crop.y > y || y >= crop.y + crop.height {
        return OUTSIDE_IMPORTANCE;
    }

    let xf = (x - crop.x) as f64 / (crop.width as f64);
    let yf = (y - crop.y) as f64 / (crop.height as f64);

    let px = (0.5 - xf).abs() * 2.0;
    let py = (0.5 - yf).abs() * 2.0;

    let dx = f64::max(px - 1.0 + EDGE_RADIUS, 0.0);
    let dy = f64::max(py - 1.0 + EDGE_RADIUS, 0.0);
    let d = (dx * dx + dy * dy) * EDGE_WEIGHT;

    let mut s = 1.41 - (px * px + py * py).sqrt();
    if RULE_OF_THIRDS {
        s += (f64::max(0.0, s + d + 0.5) * 1.2) * (thirds(px) + thirds(py))
    }

    s + d
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::strategy::Strategy;
    use proptest::test_runner::Config;

    fn gray(c: u8) -> RGB {
        RGB::new(c, c, c)
    }

    #[test]
    fn thirds_test() {
        assert_eq!(0.0, thirds(0.0));
        assert_eq!(0.0, thirds(0.5));
        assert_eq!(0.0, thirds(1.0));
        assert_eq!(1.0, thirds(1.0/3.0));
        assert_eq!(0.9288888888888889, thirds(0.9/3.0));
        assert_eq!(0.9288888888888884, thirds(1.1/3.0));
        assert_eq!(0.7155555555555557, thirds(1.2/3.0));
        assert_eq!(0.3599999999999989, thirds(1.3/3.0));
        assert_eq!(0.0, thirds(1.4/3.0));
    }

    #[test]
    fn bounds_test() {
        assert_eq!(0, bounds(-1.0));
        assert_eq!(0, bounds(0.0));
        assert_eq!(10, bounds(10.0));
        assert_eq!(255, bounds(255.0));
        assert_eq!(255, bounds(255.1));
    }

    #[test]
    fn cie_test() {
        assert_eq!(0.0, gray(0).cie());
        assert_eq!(331.49999999999994, gray(255).cie());
    }

    #[test]
    fn skin_col_test() {
        assert_eq!(0.7550795306611966, skin_col(gray(0)));
        assert_eq!(0.7550795306611966, skin_col(gray(1)));
        assert_eq!(0.7550795306611966, skin_col(gray(127)));
        assert_eq!(0.7550795306611966, skin_col(gray(34)));
        assert_eq!(0.7550795306611966, skin_col(gray(255)));
        assert_eq!(0.5904611542890027, skin_col(RGB::new(134, 45, 23)));
        assert_eq!(0.9384288009573658, skin_col(RGB::new(199, 145, 112)));
        assert_eq!(0.9380840524535538, skin_col(RGB::new(100, 72, 56)));
        assert_eq!(0.9384445374828501, skin_col(RGB::new(234, 171, 132)));
    }

    #[test]
    fn importance_tests() {
        assert_eq!(
            -6.404213562373096,
            importance(
                &Crop { x: 0, y: 0, width: 1, height: 1 },
                0,
                0)
        );
    }

    fn color() -> impl Strategy<Value=RGB> {
        (0..=255u8, 0..=255u8, 0..=255u8)
            .prop_map(|(r, g, b)| RGB{r,g,b})
    }

    fn between_0_and_1() -> impl Strategy<Value=f64> {
        (0u64..).prop_map(|i| i as f64 / u64::max_value() as f64)
    }

    proptest!{
        #![proptest_config(Config::with_cases(10000))]
        #[test]
        fn skin_col_score_is_between_0_and_1(c in color()) {
            let score = skin_col(c);

            //TODO Change 0.94 to 1.0 when values in formulas are fixed
            assert!(score >= 0.0 && score <= 0.94);
        }

        #[test]
        fn thirds_result_is_within_defined_boundaries(input in between_0_and_1()) {
            let result = thirds(input);
            if input > 0.4583333333333332 || input <= 0.2083333333333334 {
                assert_eq!(result, 0.0);
            } else {
                assert!(result > 0.0);
                assert!(result <= 1.0);
            }
        }
    }
}