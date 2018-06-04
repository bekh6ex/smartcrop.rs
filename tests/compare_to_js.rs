#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate proptest;

extern crate image;
extern crate serde;
extern crate serde_json;
extern crate smartcrop;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::tuple::TupleValueTree;
use smartcrop::*;
use std::process::Command;
use std::process::Stdio;
use std::mem;


#[cfg(feature = "compare-to-js")]
proptest! {
    #[test]
    fn produces_the_same_result_as_js_version(
        ref image in random_image1(15, 15),
        crop_w in 1u32..2,
        crop_h in 1u32..2
    ) {
        println!("Test image w={:?} h={:?}", image.width(), image.height());

        if image.get(0,0).b > 127  {
         panic!("B is big");
        }


        let analyzer = Analyzer::new(CropSettings::default());

        let js_crop_result = crop_via_js(image, crop_w, crop_h);

//        println!("{:?}", js_crop_result);

        let rust_crop = analyzer.find_best_crop(image, crop_w, crop_h).expect("Failed to find crop");

        assert_results_are_equal(rust_crop, js_crop_result);
    }
}


#[derive(Debug, Clone)]
struct TestImage {
    w: u32,
    h: u32,
    pixels: Vec<Vec<RGB>>,
}

impl TestImage {
    fn new_from_fn<G>(w: u32, h: u32, generate: G) -> TestImage
        where G: Fn(u32, u32) -> RGB {
        const WHITE: RGB = RGB { r: 255, g: 255, b: 255 };
        let mut pixels = vec![vec![WHITE; h as usize]; w as usize];

        for y in 0..h {
            for x in 0..w {
                pixels[x as usize][y as usize] = generate(x as u32, y as u32)
            }
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
    fn resize(&self, width: u32) -> TestImage {
        if width == self.w {
            return self.clone();
        }

        unimplemented!()
    }
}

fn random_color() -> BoxedStrategy<RGB> {
    (0..256u16, 0..256u16, 0..256u16)
        .prop_map(|(r, g, b)| {
            RGB::new(r as u8, g as u8, b as u8)
        })
        .boxed()
}

//TODO: Change image generation so that image shrinking would be more effective
fn random_image(max_width: u32, max_height: u32) -> BoxedStrategy<TestImage> {
    let max_size = max_width * max_height;
    let max_usize = max_size as usize;

//    prop::collection::vec(random_color(), max_usize..(max_usize + 1))
//        .prop_ind_flat_map(move |colors| {
//            (1..max_width, 1..max_height, Just(colors))
//        })
//        .prop_flat_map(|(w, h, c)|{
//
//        })
//        .prop_map(|(w, h, c)| {
//            TestImage::new_from_fn(w, h, |x, y| {
//                c[(x + y * w) as usize]
//            })
//        })
//        .boxed()

    (1..max_width, 1..max_height)
        .prop_flat_map(|(w, h)| {
            let size = (w * h) as usize;
            let colors = prop::collection::vec(random_color(), size..(size + 1));
            (Just(w), Just(h), colors)
        })
        .prop_map(|(w, h, c)| {
            TestImage::new_from_fn(w, h, |x, y| {
                c[(x + y * w) as usize]
            })
        })
        .boxed()
}

fn crop_via_js(img: &impl Image, crop_w: u32, crop_h: u32) -> JsCrop {
    let mut child = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("-i")
        .arg("smartcrop-js")
        .arg("--width")
        .arg(format!("{}", crop_w))
        .arg("--height")
        .arg(format!("{}", crop_h))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("JS failed to process image");

    let mut imgbuf = image::ImageBuffer::new(img.width(), img.height());

    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let color = img.get(x, y);
        *pixel = image::Rgb { data: [color.r, color.g, color.b] };
    }

    {
        // limited borrow of stdin
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        image::ImageRgb8(imgbuf).write_to(stdin, image::PNG);
    }

    let js_output = child
        .wait_with_output()
        .expect("failed to wait on child");

    assert!(js_output.status.success(), "JS failed to process image");
    let s = &String::from_utf8(js_output.stdout).unwrap();

    let deserialized: JsResult = serde_json::from_str(s)
        .expect("Failed to deserialize result");

    deserialized.top_crop
}

fn assert_results_are_equal(rust_crop: ScoredCrop, js_top_crop: JsCrop) {
    // Don't compare crop dimensions, since there can be many crops with the same score
    // and we don't want to test `max()` logic.

    assert_eq_f64(rust_crop.score.detail, js_top_crop.score.detail, "detail");
    assert_eq_f64(rust_crop.score.saturation, js_top_crop.score.saturation, "saturation");
    assert_eq_f64(rust_crop.score.skin, js_top_crop.score.skin, "skin");
    assert_eq_f64(rust_crop.score.total, js_top_crop.score.total, "total");
}

fn assert_eq_f64(f1: f64, f2: f64, prop: &str) {
    // Allow some deviation as soon as cast to Uint8 in JS is strange:
    // ```js
    // let u8arr = new Uint8ClampedArray(2);
    // u8arr[0] = 86.5
    // u8arr[1] = 86.500000000000007106
    // console.log(u8arr) // > Uint8ClampedArray [ 86, 87 ]
    // ```
    let delta = (f1 * 0.1).abs();

    assert!((f1 - f2).abs() <= delta, format!("{}: f1={:?} f2={:?} delta={:?}", prop, f1, f2, delta));
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct JsResult {
    top_crop: JsCrop
}

#[derive(Deserialize, Debug)]
struct JsCrop {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    score: JsScore,
}

#[derive(Deserialize, Debug)]
struct JsScore {
    detail: f64,
    saturation: f64,
    skin: f64,
    boost: f64,
    total: f64,
}

use proptest::strategy::*;
use proptest::test_runner::{TestRunner, Reason};

#[derive(Debug)]
struct TestImageStrategy {
    max_width: u32,
    max_height: u32,
}

fn random_image1(max_width: u32, max_height: u32) -> TestImageStrategy {
    TestImageStrategy { max_width, max_height }
}

impl Strategy for TestImageStrategy {
    type Value = TestImageValueTree;

    fn new_value(&self, runner: &mut TestRunner) -> Result<<Self as Strategy>::Value, Reason> {
        let width = (1u32..self.max_width).new_value(runner).unwrap().current();
        let height = (1u32..self.max_height).new_value(runner).unwrap().current();
        let max_size = (self.max_width * self.max_height) as usize;
        let colors = prop::collection::vec(random_color(), max_size..(max_size + 1))
            .new_value(runner)
            .unwrap()
            .current();

        Ok(TestImageValueTree {
            curr: TestImage::new_from_fn(width, height, |x, y| { colors[(x + width * y) as usize] }),
            prev: None,
            step: ImageSimplification::Dimension(Side::Top),
        })
    }
}

enum ImageSimplification {
    Dimension(Side),
    Color(u32, u32),
}

enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    fn next(&self) -> Option<Side> {
        use Side::*;

        match self {
            Top => Some(Right),
            Right => Some(Bottom),
            Bottom => Some(Left),
            Left => None,
        }
    }

    fn apply(&self, image: &TestImage) -> Option<TestImage> {
        match *self {
            Side::Top => {
                if image.height() == 1 {
                    None
                } else {
                    Some(TestImage::new_from_fn(image.width(), image.height() - 1, |x, y| { image.get(x, y + 1) }))
                }
            }
            Side::Right => {
                if image.width() == 1 {
                    None
                } else {
                    Some(TestImage::new_from_fn(image.width() - 1, image.height(), |x, y| { image.get(x, y) }))
                }
            }
            Side::Bottom => {
                if image.height() == 1 {
                    None
                } else {
                    Some(TestImage::new_from_fn(image.width(), image.height() - 1, |x, y| { image.get(x, y) }))
                }
            }
            Side::Left => {
                if image.width() == 1 {
                    None
                } else {
                    Some(TestImage::new_from_fn(image.width() - 1, image.height(), |x, y| { image.get(x + 1, y) }))
                }
            }

            _ => None
        }
    }
}

impl ImageSimplification {
    fn apply(&self, image: &TestImage) -> Option<TestImage> {
        match self {
            ImageSimplification::Dimension(ref side) => {
                side.apply(image)
            }
            ImageSimplification::Color(x, y) => {
                use ValueTree;
                let c = image.get(*x, *y);
                let mut tree = TupleValueTree::new((c.r, c.g, c.b));
                if tree.simplify() {
                    let c1 = tree.current();

                    Some(TestImage::new_from_fn(
                        image.width(),
                        image.height(),
                        |x1, y1| { if x1 == *x && y1 == *y { c1 } else { image.get(x1, y1) } },
                    ))
                } else {
                    unimplemented!()
                }
            }
            _ => None
        }
    }
}

struct TestImageValueTree {
    curr: TestImage,
    prev: Option<TestImage>,
    step: ImageSimplification,
}

impl TestImageValueTree {
    fn next_step(&self) -> Option<ImageSimplification> {
        use ImageSimplification::*;
        match self.step {
            Dimension(ref side) => {
                let side = side.next();
                match side {
                    Some(side) => Some(Dimension(side)),
                    None => None
                }
            }
            _ => None
        }
    }
}

impl ValueTree for TestImageValueTree {
    type Value = TestImage;

    fn current(&self) -> <Self as ValueTree>::Value {
        self.curr.clone()
    }

    fn simplify(&mut self) -> bool {
        println!("Simplify");
        match self.step {
            ImageSimplification::Dimension(side) => {
                for _ in 0..4 {
                    let simplification_result = self.step.apply(&self.curr);

                    if let Some(image) = simplification_result {
                        let prev_image = mem::replace(&mut self.curr, image);

                        self.prev = Some(prev_image);

                        return true;
                    }

                    if let Some(side) = side.next() {
                        self.step = ImageSimplification::Dimension(side);
                    } else {
                        break;
                    }
                }
            }
        }

        false
    }

    fn complicate(&mut self) -> bool {
        println!("Complicate");

        if let Some(step) = self.next_step() {
            self.step = step;
        }

        if (self.prev.is_some()) {
            let some_image = mem::replace(&mut self.prev, None);
            self.curr = some_image.unwrap();
            true
        } else {
            false
        }
//        match self.prev {
//            Some(ref image) => {
//                self.curr = *image;
//                self.prev = None;
//                true
//            }
//            None => {
//                false
//            }
//        }
    }
}
