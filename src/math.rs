
const skinColor: [f64; 3] = [0.78, 0.57, 0.44];
const outsideImportance: f64 = -0.5;
const edgeRadius: f64 = 0.4;
const edgeWeight: f64 = -20.0;
const ruleOfThirds: bool = true;


#[derive(Copy, Clone, PartialEq, Debug)]
pub struct RGB {
    pub r:u8,
    pub g:u8,
    pub b:u8
}

impl RGB {
    pub fn new(r: u8, g:u8, b:u8) -> RGB {
        RGB{r,g,b}
    }
}





// Score contains values that classify matches
#[derive(Clone, PartialEq, Debug)]
pub struct Score {
    pub Detail: f64,
    pub Saturation: f64,
    pub Skin: f64,
    pub Total: f64
}

// Crop contains results
#[derive(Clone, PartialEq, Debug)]
pub struct Crop {
    pub X: u32,
    pub Y: u32,
    pub Width: u32,
    pub Height: u32,
}

impl Crop {
    pub fn scale(&self, ratio: f64) -> Crop {
        Crop {
            X: (self.X as f64 * ratio) as u32,
            Y: (self.Y as f64 * ratio) as u32,
            Width: (self.Width as f64 * ratio) as u32,
            Height: (self.Height as f64 * ratio) as u32
        }
    }
}

#[derive(Debug)]
pub struct ScoredCrop {
    pub crop: Crop,
    pub score: Score
}

impl ScoredCrop {
    pub fn scale(&self, ratio: f64) -> ScoredCrop {
        ScoredCrop {
            crop: self.crop.scale(ratio),
            score: self.score.clone()
        }

    }
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

pub fn bounds(l: f64) -> u8 {
    f64::min(f64::max(l, 0.0), 255.0).round() as u8
}

pub fn cie(c: RGB) -> f64 {
    0.5126 * c.b as f64 + 0.7152 * c.g as f64 + 0.0722 * c.r as f64
}

pub fn skinCol(c: RGB) -> f64 {
    let mag = (c.r as f64 * c.r as f64 + c.g as f64 * c.g as f64 + c.b as f64 * c.b as f64).sqrt();
    let rd = c.r as f64 / mag - skinColor[0];
    let gd = c.g as f64 / mag - skinColor[1];
    let bd = c.b as f64 / mag - skinColor[2];

    let d = (rd * rd + gd * gd + bd * bd).sqrt();

    1.0 - d
}

pub fn saturation(c: RGB) -> f64 {
    let maximum = f64::max(f64::max(c.r as f64 / 255.0, c.g as f64 / 255.0), c.b as f64 / 255.0);
    let minimum = f64::min(f64::min(c.r as f64 / 255.0, c.g as f64 / 255.0), c.b as f64 / 255.0);


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

pub fn importance(crop: &Crop, x: u32, y: u32) -> f64 {
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

    fn gray(c: u8) -> RGB {
        RGB::new(c, c, c)
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
        assert_eq!(0, bounds(-1.0));
        assert_eq!(0, bounds(0.0));
        assert_eq!(10, bounds(10.0));
        assert_eq!(255, bounds(255.0));
        assert_eq!(255, bounds(255.1));
    }

    #[test]
    fn cie_test() {
        assert_eq!(0.0, cie(gray(0)));
        assert_eq!(331.49999999999994, cie(gray(255)));
    }

    #[test]
    fn skinCol_test() {
        assert!(skinCol(gray(0)).is_nan());
        assert_eq!(0.7550795306611965, skinCol(gray(255)));
    }

    #[test]
    fn saturation_tests() {
        assert_eq!(0.0, saturation(gray(0)));
        assert_eq!(0.0, saturation(gray(255)));
        assert_eq!(1.0, saturation(RGB::new(255, 0, 0)));
        assert_eq!(1.0, saturation(RGB::new(0, 255, 0)));
        assert_eq!(1.0, saturation(RGB::new(0, 0, 255)));
        assert_eq!(1.0, saturation(RGB::new(0, 255, 255)));
    }

    #[test]
    fn importance_tests() {
        assert_eq!(
            -6.404213562373096,
            importance(
                &Crop { X: 0, Y: 0, Width: 1, Height: 1 },
                0,
                0)
        );
    }

    #[test]
    fn crop_scale_test() {
        let crop = Crop{
            X:2,
            Y:4,
            Width:8,
            Height:16
        };

        let scaled_crop = crop.scale(0.5);

        assert_eq!(1, scaled_crop.X);
        assert_eq!(2, scaled_crop.Y);
        assert_eq!(4, scaled_crop.Width);
        assert_eq!(8, scaled_crop.Height);
    }


    fn any_score() -> Score {
        Score { Detail: 1.0, Saturation: 2.0, Skin: 3.0, Total: 6.0 }
    }

}