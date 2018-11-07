
use super::*;

const SKIN_COLOR: [f64; 3] = [0.78, 0.57, 0.44];
const OUTSIDE_IMPORTANCE: f64 = -0.5;
const EDGE_RADIUS: f64 = 0.4;
const EDGE_WEIGHT: f64 = -20.0;
const RULE_OF_THIRDS: bool = true;

// test
fn thirds(x: f64) -> f64 {
    let x = ((x - (1.0 / 3.0) + 1.0) % 2.0 * 0.5 - 0.5) * 16.0;
    return f64::max(1.0 - x * x, 0.0);
}

pub fn bounds(l: f64) -> u8 {
    f64::min(f64::max(l, 0.0), 255.0).round() as u8
}

pub fn skin_col(c: RGB) -> f64 {
    let r = c.r as f64;
    let g = c.g as f64;
    let b = c.b as f64;
    let mag = (r.powi(2) + g.powi(2)+ b.powi(2)).sqrt();
    let rd = r / mag - SKIN_COLOR[0];
    let gd = g / mag - SKIN_COLOR[1];
    let bd = b / mag - SKIN_COLOR[2];

    let d = (rd * rd + gd * gd + bd * bd).sqrt();

    1.0 - d
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

    fn gray(c: u8) -> RGB {
        RGB::new(c, c, c)
    }

    #[test]
    fn thirds_test() {
        assert_eq!(0.0, thirds(0.0));
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
        assert!(skin_col(gray(0)).is_nan());
        assert_eq!(0.7550795306611965, skin_col(gray(255)));
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

}