#![feature(test)]

extern crate smartcrop;
extern crate test;

use smartcrop::*;
use test::Bencher;

const WHITE: RGB = RGB {
    r: 255,
    g: 255,
    b: 255,
};
const GREEN: RGB = RGB { r: 0, g: 255, b: 0 };
const SKIN: RGB = RGB {
    r: 255,
    g: 200,
    b: 159,
};

#[derive(Debug, Clone)]
struct BenchImage {
    w: u32,
    h: u32,
    pixels: [[RGB; 24]; 8],
}

impl BenchImage {
    fn new_from_fn<G>(w: u32, h: u32, generate: G) -> BenchImage
    where
        G: Fn(u32, u32) -> RGB,
    {
        let mut pixels = [[WHITE; 24]; 8];

        for y in 0..h {
            for x in 0..w {
                pixels[y as usize][x as usize] = generate(x as u32, y as u32)
            }
        }

        BenchImage { w, h, pixels }
    }
}

impl Image for BenchImage {
    fn width(&self) -> u32 {
        self.w
    }

    fn height(&self) -> u32 {
        self.h
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        self.pixels[y as usize][x as usize]
    }
}

impl ResizableImage<Self> for BenchImage {
    fn resize(&self, width: u32, height: u32) -> Self {
        if width == self.w {
            return self.clone();
        }

        unimplemented!()
    }
}

#[bench]
fn bench_find_best_crop(b: &mut Bencher) {
    let image = BenchImage::new_from_fn(24, 8, |x, _y| {
        if x < 9 {
            GREEN
        } else if x < 16 {
            SKIN
        } else {
            WHITE
        }
    });

    let analyzer = Analyzer::new(CropSettings::default());
    let eight = std::num::NonZeroU32::new(8).unwrap();

    b.iter(|| {
        analyzer.find_best_crop(&image, eight, eight).unwrap();
    });
}
