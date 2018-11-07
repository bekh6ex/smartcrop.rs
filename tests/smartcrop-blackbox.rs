extern crate rand;
#[macro_use]
extern crate proptest;
extern crate smartcrop;

use smartcrop::*;
use std::num::NonZeroU32;

// All the "unobvious" numbers in tests were acquired by running same code in smartcrop.js
// Used smartcrop.js commit: 623d271ad8faf24d78f9364fcc86b5132a368576

const WHITE: RGB = RGB { r: 255, g: 255, b: 255 };
const SKIN: RGB = RGB { r: 255, g: 200, b: 159 };

#[derive(Debug, Clone)]
struct TestImage {
    w: u32,
    h: u32,
    pixels: Vec<Vec<RGB>>,
}

impl TestImage {
    fn new_from_fn<G>(w: u32, h: u32, generate: G) -> TestImage
        where G: Fn(u32, u32) -> RGB {
        let mut pixels = vec![vec![WHITE; h as usize]; w as usize];

        for y in 0..h {
            for x in 0..w {
                pixels[x as usize][y as usize] = generate(x as u32, y as u32)
            }
        }

        TestImage { w, h, pixels }
    }

    fn new_white(w: u32, h: u32) -> TestImage {
        let pixels = vec![vec![WHITE; h as usize]; w as usize];

        TestImage { w, h, pixels }
    }

    fn crop(&self, x: u32, y: u32, w: u32, h: u32) -> TestImage {
        let mut pixels = Vec::with_capacity(w as usize);
        for x1 in 0..w {
            let mut column = Vec::with_capacity(h as usize);
            for y1 in 0..h {
                let pixel = self.get(x + x1, y + y1);
                column.push(pixel)
            }
            pixels.push(column);
        }

        TestImage { w, h, pixels }
    }
}

impl Image for TestImage {
    fn width(&self) -> u32 {
        self.w
    }

    fn height(&self) -> u32 {
        self.h
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        self.pixels[x as usize][y as usize]
    }
}

impl ResizableImage<TestImage> for TestImage {
    fn resize(&self, width: u32, _height: u32) -> TestImage {
        if width == self.w {
            return self.clone();
        }

        let height = (self.h as f64 * width as f64 / self.w as f64).round() as u32;

        //TODO Implement more or less correct resizing
        return TestImage { w: width, h: height, pixels: self.pixels.clone() };
    }
}

#[test]
fn find_best_crop_test() {
    let image = TestImage::new_from_fn(
        24,
        8,
        |x, _| {
            if x < 9 {
                RGB { r: 0, g: 255, b: 0 }
            } else if x < 16 {
                SKIN
            } else {
                WHITE
            }
        },
    );
    let analyzer = Analyzer::new(CropSettings::default());

    let crop = analyzer.find_best_crop(&image, NonZeroU32::new(8).unwrap(), NonZeroU32::new(8).unwrap()).unwrap();

    assert_eq!(crop.crop.width, 8);
    assert_eq!(crop.crop.height, 8);
    assert_eq!(crop.crop.y, 0);
    assert_eq!(crop.crop.x, 16);
    assert_eq!(crop.score.detail, -4.040026482281278);
    assert_eq!(crop.score.saturation, -0.3337408688965783);
    assert_eq!(crop.score.skin, -0.13811572472126107);
    assert_eq!(crop.score.total, -0.017031057622565366);
}

#[test]
fn find_best_crop_wrong_rounding_test() {
    let image = TestImage::new_from_fn(
        640,
        426,
        |_, _| { WHITE },
    );
    let analyzer = Analyzer::new(CropSettings::default());

    let crop = analyzer.find_best_crop(&image, NonZeroU32::new(10).unwrap(), NonZeroU32::new(10).unwrap()).unwrap();

    assert_eq!(crop.crop.width, 426);
    assert_eq!(crop.crop.height, 426);
}

#[test]
fn find_best_crop_zero_sized_image_gives_error() {
    let image = TestImage::new_white(0, 0);
    let analyzer = Analyzer::new(CropSettings::default());

    let result = analyzer.find_best_crop(&image, NonZeroU32::new(1).unwrap(), NonZeroU32::new(1).unwrap());

    assert_eq!(Error::ZeroSizedImage, result.unwrap_err());
}

#[test]
// If image dimension is less than SCORE_DOWN_SAMPLE
fn find_best_crop_on_tiny_image_should_not_panic() {
    let image = TestImage::new_white(1, 1);
    let analyzer = Analyzer::new(CropSettings::default());

    let _ = analyzer.find_best_crop(&image, NonZeroU32::new(1).unwrap(), NonZeroU32::new(1).unwrap());
}


#[test]
fn result_is_as_in_js() {
    let image = TestImage { w: 8, h: 8, pixels: vec![vec![RGB { r: 83, g: 83, b: 216 }, RGB { r: 0, g: 229, b: 177 }, RGB { r: 58, g: 199, b: 58 }, RGB { r: 60, g: 56, b: 26 }, RGB { r: 0, g: 217, b: 145 }, RGB { r: 13, g: 13, b: 82 }, RGB { r: 56, g: 222, b: 56 }, RGB { r: 249, g: 62, b: 62 }], vec![RGB { r: 49, g: 49, b: 146 }, RGB { r: 0, g: 0, b: 0 }, RGB { r: 45, g: 26, b: 20 }, RGB { r: 0, g: 167, b: 215 }, RGB { r: 185, g: 44, b: 44 }, RGB { r: 221, g: 172, b: 172 }, RGB { r: 153, g: 132, b: 66 }, RGB { r: 72, g: 250, b: 72 }], vec![RGB { r: 13, g: 199, b: 13 }, RGB { r: 188, g: 42, b: 3 }, RGB { r: 41, g: 153, b: 41 }, RGB { r: 0, g: 152, b: 236 }, RGB { r: 3, g: 3, b: 143 }, RGB { r: 34, g: 121, b: 34 }, RGB { r: 243, g: 66, b: 66 }, RGB { r: 188, g: 1, b: 1 }], vec![RGB { r: 64, g: 196, b: 175 }, RGB { r: 180, g: 177, b: 127 }, RGB { r: 58, g: 58, b: 253 }, RGB { r: 117, g: 24, b: 24 }, RGB { r: 62, g: 192, b: 62 }, RGB { r: 70, g: 70, b: 204 }, RGB { r: 152, g: 10, b: 10 }, RGB { r: 41, g: 41, b: 149 }], vec![RGB { r: 122, g: 117, b: 2 }, RGB { r: 92, g: 210, b: 192 }, RGB { r: 66, g: 229, b: 66 }, RGB { r: 0, g: 0, b: 0 }, RGB { r: 73, g: 28, b: 28 }, RGB { r: 213, g: 95, b: 95 }, RGB { r: 195, g: 33, b: 33 }, RGB { r: 43, g: 24, b: 19 }], vec![RGB { r: 76, g: 35, b: 41 }, RGB { r: 184, g: 241, b: 100 }, RGB { r: 40, g: 40, b: 251 }, RGB { r: 65, g: 65, b: 28 }, RGB { r: 21, g: 18, b: 9 }, RGB { r: 32, g: 174, b: 32 }, RGB { r: 69, g: 27, b: 27 }, RGB { r: 223, g: 115, b: 115 }], vec![RGB { r: 152, g: 177, b: 197 }, RGB { r: 0, g: 0, b: 74 }, RGB { r: 33, g: 150, b: 33 }, RGB { r: 0, g: 184, b: 191 }, RGB { r: 15, g: 70, b: 15 }, RGB { r: 48, g: 40, b: 21 }, RGB { r: 21, g: 21, b: 138 }, RGB { r: 64, g: 162, b: 64 }], vec![RGB { r: 0, g: 38, b: 194 }, RGB { r: 32, g: 138, b: 32 }, RGB { r: 90, g: 7, b: 3 }, RGB { r: 86, g: 86, b: 234 }, RGB { r: 59, g: 51, b: 26 }, RGB { r: 51, g: 51, b: 22 }, RGB { r: 39, g: 39, b: 96 }, RGB { r: 59, g: 54, b: 26 }]] };

    let crop_w = NonZeroU32::new(1).unwrap();
    let crop_h = NonZeroU32::new(1).unwrap();

    let analyzer = Analyzer::new(CropSettings::default());

    let crop = analyzer.find_best_crop(&image, crop_w, crop_h).expect("Failed to find crop");

    assert_eq!(crop.score.detail, -3.7420698854650642);
    assert_eq!(crop.score.saturation, -1.713699592238245);
    assert_eq!(crop.score.skin, -0.5821112502841688);
    assert_eq!(crop.score.total, -0.030743502919192832);
}


#[test]
fn crop_is_within_the_image_boundaries_prop_test_found_case() {
    let crop_w = NonZeroU32::new(1).unwrap();
    let crop_h = NonZeroU32::new(2).unwrap();
    let image = TestImage::new_from_fn(536, 581, |x, y| {
        if x == 535 && y > 550 {
            RGB::new(255, 255, 255)
        } else {
            RGB::new(0, 0, 0)
        }
    });
    let analyzer = Analyzer::new(CropSettings::default());

    let result = analyzer.find_best_crop(&image, crop_w, crop_h);

    let crop = result.unwrap().crop;
    assert!(crop.x + crop.width <= image.width());
    assert!(crop.y + crop.height <= image.height());
}

#[test]
fn does_not_crash_when_crop_width_is_too_big_for_the_image() {
    let crop_w = NonZeroU32::new(3).unwrap();
    let crop_h = NonZeroU32::new(1).unwrap();
    let image = TestImage::new_white(1, 1);
    let analyzer = Analyzer::new(CropSettings::default());

    let result = analyzer.find_best_crop(&image, crop_w, crop_h);

    assert!(result.is_ok());
}

#[test]
fn does_not_crash_when_crop_height_is_too_big_for_the_image() {
    let crop_w = NonZeroU32::new(1).unwrap();
    let crop_h = NonZeroU32::new(3).unwrap();
    let image = TestImage::new_white(1, 1);
    let analyzer = Analyzer::new(CropSettings::default());

    let result = analyzer.find_best_crop(&image, crop_w, crop_h);

    assert!(result.is_ok());
}

mod property_testing {

    use smartcrop::*;
    use std::num::NonZeroU32;

    use proptest::prelude::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::{Reason, TestRunner};
    use rand::distributions::Distribution;
    use self::Side::*;
    use self::Simplification::*;
    use super::*;

    fn random_image(max_width: u32, max_height: u32) -> TestImageStrategy {
        TestImageStrategy::new(max_width, max_height)
    }


    proptest! {
    #[test]
    fn crop_is_within_the_image_boundaries(
        ref image in random_image(600, 600),
        crop_w in 1u32..,
        crop_h in 1u32..
    ) {
        let analyzer = Analyzer::new(CropSettings::default());

        let result = analyzer.find_best_crop(image, NonZeroU32::new(crop_w).unwrap(), NonZeroU32::new(crop_h).unwrap());

        let crop = result.unwrap().crop;
        assert!(crop.x + crop.width <= image.width());
        assert!(crop.y + crop.height <= image.height());
    }
}

    #[derive(Debug)]
    struct TestImageStrategy {
        max_w: u32,
        max_h: u32,
    }

    impl TestImageStrategy {
        fn new(max_w: u32, max_h: u32) -> TestImageStrategy {
            TestImageStrategy { max_w, max_h }
        }
    }

    impl Strategy for TestImageStrategy {
        type Tree = TestImageValueTree;
        type Value = TestImage;

        fn new_tree(&self, runner: &mut TestRunner) -> Result<<Self as Strategy>::Tree, Reason> {
            let w = rand::distributions::Uniform::new(1, self.max_w + 1).sample(runner.rng());
            let h = rand::distributions::Uniform::new(1, self.max_h + 1).sample(runner.rng());

            let mut pixels = Vec::with_capacity(w as usize);
            for _ in 0..w {
                let mut column = Vec::with_capacity(h as usize);
                for _ in 0..h {
                    let r = runner.rng().gen();
                    let g = runner.rng().gen();
                    let b = runner.rng().gen();
                    column.push(RGB { r, g, b });
                };
                pixels.push(column);
            };
            let image = TestImage { w, h, pixels };

            Ok(TestImageValueTree::new(image))
        }
    }

    enum Side {
        Top,
        Right,
        Bottom,
        Left,
    }

    enum Simplification {
        Cut(Side),
        Darken { x: u32, y: u32 },
    }

    struct TestImageValueTree {
        images: Vec<TestImage>,
        simplification: Option<Simplification>,
    }

    impl TestImageValueTree {
        fn new(image: TestImage) -> TestImageValueTree {
            TestImageValueTree {
                images: vec![image],
                simplification: Some(Cut(Top)),
            }
        }

        fn switch_to_next_simplification(&mut self) {
            match self.simplification {
                Some(Cut(Top)) => self.simplification = Some(Cut(Right)),
                Some(Cut(Right)) => self.simplification = Some(Cut(Bottom)),
                Some(Cut(Bottom)) => self.simplification = Some(Cut(Left)),
                Some(Cut(Left)) => self.simplification = Some(Darken { x: 0, y: 0 }),
                Some(Darken { x, y }) => {
                    let image = self.images.last().unwrap();
                    self.simplification = if y == image.height() - 1 {
                        None
                    } else if x == image.width() - 1 {
                        Some(Darken { x: 0, y: y + 1 })
                    } else {
                        Some(Darken { x: x + 1, y })
                    }
                }
                _ => self.simplification = None,
            }
        }

        fn can_simplify(&mut self) -> bool {
            match self.simplification {
                Some(Cut(Top)) | Some(Cut(Bottom)) => {
                    let image_height = self.images.last().unwrap().height();
                    if image_height > 1 {
                        true
                    } else {
                        self.switch_to_next_simplification();
                        self.can_simplify()
                    }
                }
                Some(Cut(Left)) | Some(Cut(Right)) => {
                    let image_width = self.images.last().unwrap().width();
                    if image_width > 1 {
                        true
                    } else {
                        self.switch_to_next_simplification();
                        self.can_simplify()
                    }
                }
                Some(Darken { x, y }) => {
                    let image = self.images.last().unwrap();
                    image.width() > x || image.height() > y
                }
                _ => false,
            }
        }
    }

    impl ValueTree for TestImageValueTree {
        type Value = TestImage;

        fn current(&self) -> <Self as ValueTree>::Value {
            self.images.last().unwrap().clone()
        }

        fn simplify(&mut self) -> bool {
            if !self.can_simplify() {
                eprintln!("Cannot simplify");
                return false;
            }

            let simplified_image = {
                match self.simplification {
                    Some(Cut(Top)) => {
                        eprintln!("Simplify Cut(Top)");
                        let image = self.images.last().unwrap();
                        image.crop(0, 1, image.w, image.h - 1)
                    }
                    Some(Cut(Right)) => {
                        eprintln!("Simplify Cut(Right)");
                        let image = self.images.last().unwrap();
                        image.crop(0, 0, image.w - 1, image.h)
                    }
                    Some(Cut(Bottom)) => {
                        eprintln!("Simplify Cut(Bottom)");
                        let image = self.images.last().unwrap();
                        image.crop(0, 0, image.w, image.h - 1)
                    }
                    Some(Cut(Left)) => {
                        eprintln!("Simplify Cut(Left)");
                        let image = self.images.last().unwrap();
                        image.crop(1, 0, image.w - 1, image.h)
                    }
                    Some(Darken { x, y }) => {
                        eprintln!("Simplify Darken {{ {}, {} }}", x, y);
                        self.switch_to_next_simplification();
                        let image = self.images.last().unwrap();
                        let mut new_image = image.clone();
                        new_image.pixels[x as usize][y as usize] = RGB { r: 0, g: 0, b: 0 };
                        new_image
                    }
                    _ => unimplemented!()
                }
            };

            self.images.push(simplified_image);

            while self.images.len() > 10 {
                self.images.remove(0);
            }

            eprintln!("Simplified");
            true
        }

        fn complicate(&mut self) -> bool {
            if self.images.len() == 1 {
                eprintln!("Cannot complicate");
                false
            } else {
                eprintln!("Complicated");

                self.images.pop();
                self.switch_to_next_simplification();
                true
            }
        }
    }
}
