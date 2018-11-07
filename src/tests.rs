use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Reason, TestRunner};
use rand;
use rand::distributions::IndependentSample;
use self::Side::*;
use self::Simplification::*;
use super::*;

// All the "unobvious" numbers in tests were acquired by running same code in smartcrop.js
// Used smartcrop.js commit: 623d271ad8faf24d78f9364fcc86b5132a368576

const WHITE: RGB = RGB { r: 255, g: 255, b: 255 };
const BLACK: RGB = RGB { r: 0, g: 0, b: 0 };
const RED: RGB = RGB { r: 255, g: 0, b: 0 };
const GREEN: RGB = RGB { r: 0, g: 255, b: 0 };
const BLUE: RGB = RGB { r: 0, g: 0, b: 255 };
const SKIN: RGB = RGB { r: 255, g: 200, b: 159 };

#[derive(Debug, Clone)]
struct TestImage {
    w: u32,
    h: u32,
    pixels: Vec<Vec<RGB>>,
}

impl TestImage {
    fn new(w: u32, h: u32, pixels: Vec<Vec<RGB>>) -> TestImage {
        TestImage { w, h, pixels }
    }

    fn new_single_pixel(pixel: RGB) -> TestImage {
        TestImage { w: 1, h: 1, pixels: vec![vec![pixel]] }
    }

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

impl ImageMap {
    fn from_image<I: Image>(image: &I) -> ImageMap {
        let mut image_map = ImageMap::new(image.width(), image.height());

        for y in 0..image.height() {
            for x in 0..image.width() {
                let color = image.get(x, y);
                image_map.set(x, y, color);
            }
        }

        image_map
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

#[derive(Clone, Debug)]
struct SingleColorImage {
    w: u32,
    h: u32,
    color: RGB,
}

impl SingleColorImage {
    fn white(w: u32, h: u32) -> SingleColorImage {
        SingleColorImage { w, h, color: RGB::new(255, 255, 255) }
    }
}

impl Image for SingleColorImage {
    fn width(&self) -> u32 { self.w }

    fn height(&self) -> u32 { self.h }

    fn get(&self, _x: u32, _y: u32) -> RGB { self.color }
}

impl ResizableImage<SingleColorImage> for SingleColorImage {
    fn resize(&self, width: u32, _height: u32) -> SingleColorImage {
        if width == self.w {
            return self.clone();
        }

        let height = (self.h as f64 * width as f64 / self.w as f64).round() as u32;

        SingleColorImage { w: width, h: height, color: self.color }
    }
}

#[test]
fn image_map_test() {
    let mut image_map = ImageMap::new(1, 2);

    assert_eq!(image_map.width, 1);
    assert_eq!(image_map.height, 2);

    assert_eq!(image_map.get(0, 0), RGB::new(255, 255, 255));
    assert_eq!(image_map.get(0, 1), RGB::new(255, 255, 255));

    let red = RGB::new(255, 0, 0);
    image_map.set(0, 0, red);
    assert_eq!(image_map.get(0, 0), red);

    let green = RGB::new(0, 255, 0);
    image_map.set(0, 1, green);
    assert_eq!(image_map.get(0, 1), green);
}

#[test]
fn crops_test() {
    let real_min_scale = MIN_SCALE;

    let crops = crops(&ImageMap::new(8, 8), 8, 8, real_min_scale);

    assert_eq!(crops[0], Crop { x: 0, y: 0, width: 8, height: 8 })
}

#[test]
fn score_test_image_with_single_black_pixel_then_score_is_zero() {
    let mut i = ImageMap::new(1, 1);
    i.set(0, 0, RGB::new(0, 0, 0));

    let s = score(&i, &Crop { x: 0, y: 0, width: 1, height: 1 });

    assert_eq!(s, Score { detail: 0.0, saturation: 0.0, skin: 0.0, total: 0.0 });
}

#[test]
fn score_test_image_with_single_white_pixel_then_score_is_the_same_as_for_js_version() {
    let mut i = ImageMap::new(1, 1);
    i.set(0, 0, RGB::new(255, 255, 255));

    let s = score(&i, &Crop { x: 0, y: 0, width: 1, height: 1 });

    let js_version_score = Score {
        detail: -6.404213562373096,
        saturation: -7.685056274847715,
        skin: -6.468255697996827,
        total: -13.692208596353678,
    };

    assert_eq!(s, js_version_score);
}

#[test]
fn skin_detect_single_pixel_test() {
    let detect_pixel = |color: RGB| {
        let image = TestImage::new_single_pixel(color);
        let mut o = ImageMap::new(1, 1);
        o.set(0, 0, color);

        skin_detect(&image, &mut o);
        o.get(0, 0)
    };

    assert_eq!(detect_pixel(WHITE), RGB::new(0, 255, 255));
    assert_eq!(detect_pixel(BLACK), RGB::new(0, 0, 0));
    assert_eq!(detect_pixel(RED), RGB::new(0, 0, 0));
    assert_eq!(detect_pixel(GREEN), RGB::new(0, 255, 0));
    assert_eq!(detect_pixel(BLUE), RGB::new(0, 0, 255));
    assert_eq!(detect_pixel(SKIN), RGB::new(159, 200, 159));
}

#[test]
fn edge_detect_single_pixel_image_test() {
    let edge_detect_pixel = |color: RGB| {
        let image = TestImage::new_single_pixel(color);
        let mut o = ImageMap::new(1, 1);
        o.set(0, 0, color);

        edge_detect(&image, &mut o);

        o.get(0, 0)
    };

    assert_eq!(edge_detect_pixel(BLACK), BLACK);
    assert_eq!(edge_detect_pixel(WHITE), WHITE);
    assert_eq!(edge_detect_pixel(RED), RGB::new(255, 18, 0));
    assert_eq!(edge_detect_pixel(GREEN), RGB::new(0, 182, 0));
    assert_eq!(edge_detect_pixel(BLUE), RGB::new(0, 131, 255));
    assert_eq!(edge_detect_pixel(SKIN), RGB::new(255, 243, 159));
}

#[test]
fn edge_detect_3x3() {
    let image = TestImage::new(
        3,
        3,
        vec![
            vec![RED, GREEN, BLUE],
            vec![GREEN, BLUE, RED],
            vec![BLUE, RED, GREEN],
        ],
    );
    let mut o = ImageMap::new(3, 3);

    edge_detect(&image, &mut o);

    assert_eq!(o.get(0, 0), RGB { r: 255, g: 18, b: 0 });
    assert_eq!(o.get(0, 0), RGB { r: 255, g: 18, b: 0 });
    assert_eq!(o.get(1, 0), RGB { r: 0, g: 182, b: 0 });
    assert_eq!(o.get(2, 0), RGB { r: 0, g: 131, b: 255 });
    assert_eq!(o.get(0, 1), RGB { r: 0, g: 182, b: 0 });
    assert_eq!(o.get(1, 1), RGB { r: 0, g: 121, b: 255 });
    assert_eq!(o.get(2, 1), RGB { r: 255, g: 18, b: 0 });
    assert_eq!(o.get(0, 2), RGB { r: 0, g: 131, b: 255 });
    assert_eq!(o.get(1, 2), RGB { r: 255, g: 18, b: 0 });
    assert_eq!(o.get(2, 2), RGB { r: 0, g: 182, b: 0 });
}

#[test]
fn saturation_detect_3x3() {
    let image = TestImage::new(
        3,
        3,
        vec![
            vec![RED, GREEN, BLUE],
            vec![WHITE, SKIN, BLACK],
            vec![BLUE, RED, GREEN],
        ],
    );
    let mut o = ImageMap::from_image(&image);

    saturation_detect(&image, &mut o);

    assert_eq!(o.get(0, 0), RGB { r: 255, g: 0, b: 255 });
    assert_eq!(o.get(0, 1), RGB { r: 0, g: 255, b: 255 });
    assert_eq!(o.get(0, 2), RGB { r: 0, g: 0, b: 255 });
    assert_eq!(o.get(1, 0), RGB { r: 255, g: 255, b: 0 });
    assert_eq!(o.get(1, 1), RGB { r: 255, g: 200, b: 0 });
    assert_eq!(o.get(1, 2), RGB { r: 0, g: 0, b: 0 });
    assert_eq!(o.get(2, 0), RGB { r: 0, g: 0, b: 255 });
    assert_eq!(o.get(2, 1), RGB { r: 255, g: 0, b: 255 });
    assert_eq!(o.get(2, 2), RGB { r: 0, g: 255, b: 255 });
}

#[test]
fn analyze_test() {
    let image = TestImage::new_from_fn(
        24,
        24,
        |x, y| {
            if x >= 8 && x < 16 && y >= 8 && y < 16 {
                SKIN
            } else {
                WHITE
            }
        },
    );

    let crop = analyse(&CropSettings::default(), &image, 8, 8, 1.0);

    assert_eq!(crop.crop.width, 8);
    assert_eq!(crop.crop.height, 8);
    assert_eq!(crop.crop.x, 8);
    assert_eq!(crop.crop.y, 8);
    assert_eq!(crop.score.saturation, 0.0);
    assert_eq!(crop.score.detail, -1.7647058823529413);
    assert_eq!(crop.score.skin, -0.03993215515362048);
    assert_eq!(crop.score.total, -0.006637797746048519);
}

#[test]
fn find_best_crop_test() {
    let image = TestImage::new_from_fn(
        24,
        8,
        |x, _| {
            if x < 9 {
                GREEN
            } else if x < 16 {
                SKIN
            } else {
                WHITE
            }
        },
    );
    let analyzer = Analyzer::new(CropSettings::default());

    let crop = analyzer.find_best_crop(&image, 8, 8).unwrap();

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

    let crop = analyzer.find_best_crop(&image, 10, 10).unwrap();

    assert_eq!(crop.crop.width, 426);
    assert_eq!(crop.crop.height, 426);
}

#[test]
fn find_best_crop_zerosized_image_gives_error() {
    let image = TestImage::new_white(0, 0);
    let analyzer = Analyzer::new(CropSettings::default());

    let result = analyzer.find_best_crop(&image, 1, 1);

    assert_eq!(Error::ZeroSizedImage, result.unwrap_err());
}

#[test]
// If image dimension is less than SCORE_DOWN_SAMPLE
fn find_best_crop_on_tiny_image_should_not_panic() {
    let image = TestImage::new_white(1, 1);
    let analyzer = Analyzer::new(CropSettings::default());

    let _ = analyzer.find_best_crop(&image, 1, 1);
}


#[test]
fn crop_scale_test() {
    let crop = Crop {
        x: 2,
        y: 4,
        width: 8,
        height: 16,
    };

    let scaled_crop = crop.scale(0.5);

    assert_eq!(1, scaled_crop.x);
    assert_eq!(2, scaled_crop.y);
    assert_eq!(4, scaled_crop.width);
    assert_eq!(8, scaled_crop.height);
}

#[test]
fn down_sample_test() {
    let image = TestImage::new(
        3,
        3,
        vec![
            vec![RED, GREEN, BLUE],
            vec![SKIN, BLUE, RED],
            vec![BLUE, RED, GREEN],
        ],
    );

    let image_map = ImageMap::from_image(&image);

    let result = image_map.down_sample(3);

    assert_eq!(result.width, 1);
    assert_eq!(result.height, 1);
    assert_eq!(result.get(0, 0), RGB::new(184, 132, 103));
}

#[test]
fn result_is_as_in_js() {
    let image = TestImage { w: 8, h: 8, pixels: vec![vec![RGB { r: 83, g: 83, b: 216 }, RGB { r: 0, g: 229, b: 177 }, RGB { r: 58, g: 199, b: 58 }, RGB { r: 60, g: 56, b: 26 }, RGB { r: 0, g: 217, b: 145 }, RGB { r: 13, g: 13, b: 82 }, RGB { r: 56, g: 222, b: 56 }, RGB { r: 249, g: 62, b: 62 }], vec![RGB { r: 49, g: 49, b: 146 }, RGB { r: 0, g: 0, b: 0 }, RGB { r: 45, g: 26, b: 20 }, RGB { r: 0, g: 167, b: 215 }, RGB { r: 185, g: 44, b: 44 }, RGB { r: 221, g: 172, b: 172 }, RGB { r: 153, g: 132, b: 66 }, RGB { r: 72, g: 250, b: 72 }], vec![RGB { r: 13, g: 199, b: 13 }, RGB { r: 188, g: 42, b: 3 }, RGB { r: 41, g: 153, b: 41 }, RGB { r: 0, g: 152, b: 236 }, RGB { r: 3, g: 3, b: 143 }, RGB { r: 34, g: 121, b: 34 }, RGB { r: 243, g: 66, b: 66 }, RGB { r: 188, g: 1, b: 1 }], vec![RGB { r: 64, g: 196, b: 175 }, RGB { r: 180, g: 177, b: 127 }, RGB { r: 58, g: 58, b: 253 }, RGB { r: 117, g: 24, b: 24 }, RGB { r: 62, g: 192, b: 62 }, RGB { r: 70, g: 70, b: 204 }, RGB { r: 152, g: 10, b: 10 }, RGB { r: 41, g: 41, b: 149 }], vec![RGB { r: 122, g: 117, b: 2 }, RGB { r: 92, g: 210, b: 192 }, RGB { r: 66, g: 229, b: 66 }, RGB { r: 0, g: 0, b: 0 }, RGB { r: 73, g: 28, b: 28 }, RGB { r: 213, g: 95, b: 95 }, RGB { r: 195, g: 33, b: 33 }, RGB { r: 43, g: 24, b: 19 }], vec![RGB { r: 76, g: 35, b: 41 }, RGB { r: 184, g: 241, b: 100 }, RGB { r: 40, g: 40, b: 251 }, RGB { r: 65, g: 65, b: 28 }, RGB { r: 21, g: 18, b: 9 }, RGB { r: 32, g: 174, b: 32 }, RGB { r: 69, g: 27, b: 27 }, RGB { r: 223, g: 115, b: 115 }], vec![RGB { r: 152, g: 177, b: 197 }, RGB { r: 0, g: 0, b: 74 }, RGB { r: 33, g: 150, b: 33 }, RGB { r: 0, g: 184, b: 191 }, RGB { r: 15, g: 70, b: 15 }, RGB { r: 48, g: 40, b: 21 }, RGB { r: 21, g: 21, b: 138 }, RGB { r: 64, g: 162, b: 64 }], vec![RGB { r: 0, g: 38, b: 194 }, RGB { r: 32, g: 138, b: 32 }, RGB { r: 90, g: 7, b: 3 }, RGB { r: 86, g: 86, b: 234 }, RGB { r: 59, g: 51, b: 26 }, RGB { r: 51, g: 51, b: 22 }, RGB { r: 39, g: 39, b: 96 }, RGB { r: 59, g: 54, b: 26 }]] };

    let crop_w = 1;
    let crop_h = 1;

    let analyzer = Analyzer::new(CropSettings::default());

    let crop = analyzer.find_best_crop(&image, crop_w, crop_h).expect("Failed to find crop");

    assert_eq!(crop.score.detail, -3.7420698854650642);
    assert_eq!(crop.score.saturation, -1.713699592238245);
    assert_eq!(crop.score.skin, -0.5821112502841688);
    assert_eq!(crop.score.total, -0.030743502919192832);
}

#[test]
fn test_analyze_is_alwayse_called_with_one_crop_dimension_equal_to_image_dimention() {
    // assert!(img.width() == crop_width || img.height() == crop_height) was failing with this data
    let image = SingleColorImage::white(510, 211);
    let crop_w = 0;
    let crop_h = 47961520;

    let analyzer = Analyzer::new(CropSettings::default());

    let _crop = analyzer.find_best_crop(&image, crop_w, crop_h);
}


fn white_image(max_dimension: u32) -> BoxedStrategy<SingleColorImage> {
    (0..max_dimension, 0..max_dimension)
        .prop_map(|(w, h)| SingleColorImage { w, h, color: RGB::new(255, 255, 255) })
        .boxed()
}

fn _random_image(max_width: u32, max_height: u32) -> BoxedStrategy<TestImage> {
    (1..max_width, 1..max_height)
        .prop_flat_map(|(w, h)| {
            let size = (w * h * 3) as usize;
            let colors = prop::collection::vec(0..256u16, size..(size + 1));
            (Just(w), Just(h), colors)
        })
        .prop_map(|(w, h, c)| {
            TestImage::new_from_fn(w, h, |x, y| {
                let i = (x + y * w) as usize;
                RGB::new(c[i] as u8, c[i + 1] as u8, c[i + 2] as u8)
            })
        })
        .boxed()
}

fn random_image(max_width: u32, max_height: u32) -> TestImageStrategy {
    TestImageStrategy::new(max_width, max_height)
}


proptest! {
    #[test]
    fn doesnt_crash(
        ref image in white_image(2000),
        crop_w in 0u32..,
        crop_h in 0u32..
    ) {
        let analyzer = Analyzer::new(CropSettings::default());

        let _crop = analyzer.find_best_crop(image, crop_w, crop_h);
    }

    #[test]
    fn crop_is_within_the_image_boundaries(
        ref image in random_image(600, 600),
        crop_w in 1u32..,
        crop_h in 1u32..
    ) {
        let analyzer = Analyzer::new(CropSettings::default());

        let result = analyzer.find_best_crop(image, crop_w, crop_h);

        let crop = result.unwrap().crop;
        assert!(crop.x + crop.width <= image.width());
        assert!(crop.y + crop.height <= image.height());
    }
}


#[test]
fn crop_is_within_the_image_boundaries_prop_test_found_case() {
    let crop_w = 1;
    let crop_h = 2;
    let image = TestImage::new_from_fn(536, 581, |x, y| {
        if x == 535 && y > 550 {
            RGB::new(255,255,255)
        } else {
            RGB::new(0,0,0)
        }
    });
    let analyzer = Analyzer::new(CropSettings::default());

    let result = analyzer.find_best_crop(&image, crop_w, crop_h);

    let crop = result.unwrap().crop;
    assert!(crop.x + crop.width <= image.width());
    assert!(crop.y + crop.height <= image.height());
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
        let w = rand::distributions::Range::new(1, self.max_w + 1)
            .ind_sample(runner.rng());
        let h = rand::distributions::Range::new(1, self.max_h + 1)
            .ind_sample(runner.rng());

        let mut pixels = Vec::with_capacity(w as usize);
        for _ in 0..w {
            let mut column = Vec::with_capacity(h as usize);
            for _ in 0..h {
                let r = runner.rng().gen();
                let g = runner.rng().gen();
                let b = runner.rng().gen();
                column.push(RGB { r, g, b })
            }
            pixels.push(column);
        }
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
