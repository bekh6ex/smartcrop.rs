use proptest::prelude::*;
use smartcrop::*;
use proptest::test_runner::Config;
use image;
use rand;
use std::fs;
use std::process::Command;
use std::io::Write;
use std::process::Stdio;

extern crate serde;
extern crate serde_json;

const WHITE: RGB = RGB { r: 255, g: 255, b: 255 };

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

        let height = (self.h as f64 * width as f64 / self.w as f64).round() as u32;

        //TODO Implement more or less correct resizing
        return TestImage { w: width, h: height, pixels: self.pixels.clone() };
    }
}

fn random_color() -> BoxedStrategy<RGB> {
    (0..256u16, 0..256u16, 0..256u16)
        .prop_map(|(r, g, b)| {
            RGB::new(r as u8, g as u8, b as u8)
        })
        .boxed()
}

proptest! {
    #![proptest_config(Config::with_cases(1))]

    #[test]
    fn doesnt_crash_with_random_image(
        ref image in random_image(30, 30),
        crop_w in 1u32..30,
        crop_h in 1u32..30
    ) {

        let analyzer = Analyzer::new(CropSettings::default());

        let rust_crop = analyzer.find_best_crop(image, crop_w, crop_h).expect("Failed to find crop");

        let js_crop_result = crop_via_js(image, crop_w, crop_h);

        assert_results_are_equal(rust_crop, js_crop_result);
    }
}


//TODO: Change image generation so that image shrinking would be more effective
fn random_image(max_width: u32, max_height: u32) -> BoxedStrategy<TestImage> {
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
    let mut child = Command::new("nodejs")
        .arg("index.js")
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
    // Allow 5% deviation as soon as cast to U8 in JS is strange:
    // ```js
    // let u8arr = new Uint8ClampedArray(2);
    // u8arr[0] = 86.5
    // u8arr[1] = 86.51
    // console.log(u8arr) // > Uint8ClampedArray [ 86, 87 ]
    // ```
    let delta = (f1 * 0.05).abs();

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