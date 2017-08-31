
const skinColor: [f64; 3] = [0.78, 0.57, 0.44];
const outsideImportance: f64 = -0.5;
const edgeRadius: f64 = 0.4;
const edgeWeight: f64 = -20.0;
const ruleOfThirds: bool = true;


trait Color {
    fn RGB(&self) -> (f64, f64, f64);
}


// Score contains values that classify matches
struct Score {
    Detail: f64,
    Saturation: f64,
    Skin: f64,
    Total: f64
}

// Crop contains results
struct Crop {
    X: u32,
    Y: u32,
    Width: u32,
    Height: u32,
    Score: Score
}

pub fn chop(x: f64) -> f64 {
    if x < 0.0 {
        x.ceil()
    } else {
        x.floor()
    }
}


// test
fn thirds(x: f64) -> f64 {
    let x = ((x - (1.0 / 3.0) + 1.0) % 2.0 * 0.5 - 0.5) * 16.0;
    return f64::max(1.0 - x * x, 0.0);
}

fn bounds(l: f64) -> f64 {
    f64::min(f64::max(l, 0.0), 255.0)
}

fn cie(c: &Color) -> f64 {
    let (r, g, b) = c.RGB();

    0.5126 * b + 0.7152 * g + 0.0722 * r
}

fn skinCol(c: &Color) -> f64 {
    let (r, g, b) = c.RGB();

    let mag = (r * r + g * g + b * b).sqrt();
    let rd = r / mag - skinColor[0];
    let gd = g / mag - skinColor[1];
    let bd = b / mag - skinColor[2];

    let d = (rd * rd + gd * gd + bd * bd).sqrt();

    1.0 - d
}

fn saturation(c: &Color) -> f64 {
    let (r, g, b) = c.RGB();

    let maximum = f64::max(f64::max(r / 255.0, g / 255.0), b / 255.0);
    let minimum = f64::min(f64::min(r / 255.0, g / 255.0), b / 255.0);


    if maximum == minimum {
        return 0.0;
    }

    let l = (maximum + minimum) / 2.0;
    let d = maximum - minimum;

    if l > 0.5 {
        d / (2.0 - maximum - minimum)
    } else {
        d / (maximum + minimum)
    }
}

fn importance(crop: &Crop, x: u32, y: u32) -> f64 {
    if (crop.X > x || x >= crop.X + crop.Width || crop.Y > y || y >= crop.Y + crop.Height) {
        return outsideImportance;
    }

    let xf = (x - crop.X) as f64 / (crop.Width as f64);
    let yf = (y - crop.Y) as f64 / (crop.Height as f64);

    let px = (0.5 - xf).abs() * 2.0;
    let py = (0.5 - yf).abs() * 2.0;

    let dx = f64::max(px - 1.0 + edgeRadius, 0.0);
    let dy = f64::max(py - 1.0 + edgeRadius, 0.0);
    let d = (dx * dx + dy * dy) * edgeWeight;

    let mut s = 1.41 - (px * px + py * py).sqrt();
    if ruleOfThirds {
        s += (f64::max(0.0, s + d + 0.5) * 1.2) * (thirds(px) + thirds(py))
    }

    s + d
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Gray(u8);

    impl Color for Gray {
        fn RGB(&self) -> (f64, f64, f64) {
            (self.0 as f64, self.0 as f64, self.0 as f64)
        }
    }

    struct RGB(u8, u8, u8);

    impl Color for RGB {
        fn RGB(&self) -> (f64, f64, f64) {
            (self.0 as f64, self.1 as f64, self.2 as f64)
        }
    }

    #[test]
    fn chop_test() {
        assert_eq!(1.0, chop(1.1));
        assert_eq!(-1.0, chop(-1.1));
    }

    #[test]
    fn thirds_test() {
        assert_eq!(0.0, thirds(0.0));
    }

    #[test]
    fn bounds_test() {
        assert_eq!(0.0, bounds(-1.0));
        assert_eq!(0.0, bounds(0.0));
        assert_eq!(10.0, bounds(10.0));
        assert_eq!(255.0, bounds(255.0));
        assert_eq!(255.0, bounds(255.1));
    }

    #[test]
    fn cie_test() {
        assert_eq!(0.0, cie(&Gray(0)));
        assert_eq!(331.49999999999994, cie(&Gray(255)));
    }

    #[test]
    fn skinCol_test() {
        assert!(skinCol(&Gray(0)).is_nan());
        assert_eq!(0.7550795306611965, skinCol(&Gray(255)));
    }

    #[test]
    fn saturation_tests() {
        assert_eq!(0.0, saturation(&Gray(0)));
        assert_eq!(0.0, saturation(&Gray(255)));
        assert_eq!(1.0, saturation(&RGB(255, 0, 0)));
        assert_eq!(1.0, saturation(&RGB(0, 255, 0)));
        assert_eq!(1.0, saturation(&RGB(0, 0, 255)));
        assert_eq!(1.0, saturation(&RGB(0, 255, 255)));
    }

    #[test]
    fn importance_tests() {
        assert_eq!(
            -6.404213562373096,
            importance(
                &Crop { X: 0, Y: 0, Width: 1, Height: 1,
                    Score: Score { Detail: 1.0, Saturation: 2.0, Skin: 3.0, Total: 6.0 } },
                0,
                0)
        );
    }
}