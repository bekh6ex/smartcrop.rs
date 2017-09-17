#![feature(test)]

extern crate smartcrop;
extern crate test;

use test::Bencher;
use smartcrop::*;


const WHITE: RGB = RGB { r: 255, g: 255, b: 255 };
const BLACK: RGB = RGB { r: 0, g: 0, b: 0 };
const RED: RGB = RGB { r: 255, g: 0, b: 0 };
const GREEN: RGB = RGB { r: 0, g: 255, b: 0 };
const BLUE: RGB = RGB { r: 0, g: 0, b: 255 };
const SKIN: RGB = RGB { r: 255, g: 200, b: 159 };

#[derive(Debug)]
struct SinglePixelImage {
    pixel: RGB
}

#[derive(Debug, Clone)]
struct BenchImage {
    w: u32,
    h: u32,
    pixels: Vec<Vec<RGB>>
}

impl BenchImage {
    fn new(w: u32, h: u32, pixels: Vec<Vec<RGB>>) -> BenchImage {
        BenchImage { w, h, pixels }
    }
    fn new_from_fn<G>(w: u32, h: u32, generate: G) -> BenchImage
        where G: Fn(u32, u32) -> RGB {
        let mut pixels = vec![vec![WHITE; h as usize]; w as usize];

        for y in 0..h {
            for x in 0..w {
                pixels[x as usize][y as usize] = generate(x as u32, y as u32)
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

    fn resize(&self, width: u32) -> Box<Image> {
        if width == self.w {
            return Box::new(self.clone());
        }

        unimplemented!()
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        self.pixels[x as usize][y as usize]
    }
}

#[bench]
fn bench_find_best_crop(b: &mut Bencher) {
    let image = BenchImage::new_from_fn(
        24,
        8,
        |x, y| {
            if x < 9 {
                GREEN
            } else if x < 16 {
                SKIN
            } else {
                WHITE
            }
        }
    );
    let analyzer = Analyzer::new(CropSettings::default());


    b.iter(|| {
        analyzer.find_best_crop(&image, 8, 8).unwrap();
    });
}

