use super::*;

// All the "unobvious" numbers in tests were acquired by running same code in smartcrop.js
// Used smartcrop.js commit: 623d271ad8faf24d78f9364fcc86b5132a368576

const WHITE: RGB = RGB {
    r: 255,
    g: 255,
    b: 255,
};
const BLACK: RGB = RGB { r: 0, g: 0, b: 0 };
const RED: RGB = RGB { r: 255, g: 0, b: 0 };
const GREEN: RGB = RGB { r: 0, g: 255, b: 0 };
const BLUE: RGB = RGB { r: 0, g: 0, b: 255 };
const SKIN: RGB = RGB {
    r: 255,
    g: 200,
    b: 159,
};

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
        TestImage {
            w: 1,
            h: 1,
            pixels: vec![vec![pixel]],
        }
    }

    fn new_from_fn<G>(w: u32, h: u32, generate: G) -> TestImage
    where
        G: Fn(u32, u32) -> RGB,
    {
        let mut pixels = vec![vec![WHITE; h as usize]; w as usize];

        for y in 0..h {
            for x in 0..w {
                pixels[x as usize][y as usize] = generate(x as u32, y as u32)
            }
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
        return TestImage {
            w: width,
            h: height,
            pixels: self.pixels.clone(),
        };
    }
}

#[test]
fn saturation_tests() {
    assert_eq!(0.0, BLACK.saturation());
    assert_eq!(0.0, WHITE.saturation());
    assert_eq!(1.0, RGB::new(255, 0, 0).saturation());
    assert_eq!(1.0, RGB::new(0, 255, 0).saturation());
    assert_eq!(1.0, RGB::new(0, 0, 255).saturation());
    assert_eq!(1.0, RGB::new(0, 255, 255).saturation());
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

    assert_eq!(
        crops[0],
        Crop {
            x: 0,
            y: 0,
            width: 8,
            height: 8
        }
    )
}

#[test]
fn score_test_image_with_single_black_pixel_then_score_is_zero() {
    let mut i = ImageMap::new(1, 1);
    i.set(0, 0, RGB::new(0, 0, 0));

    let s = score(
        &i,
        &Crop {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
    );

    assert_eq!(
        s,
        Score {
            detail: 0.0,
            saturation: 0.0,
            skin: 0.0,
            total: 0.0
        }
    );
}

#[test]
fn score_test_image_with_single_white_pixel_then_score_is_the_same_as_for_js_version() {
    let mut i = ImageMap::new(1, 1);
    i.set(0, 0, RGB::new(255, 255, 255));

    let s = score(
        &i,
        &Crop {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
    );

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

    assert_eq!(
        o.get(0, 0),
        RGB {
            r: 255,
            g: 18,
            b: 0
        }
    );
    assert_eq!(
        o.get(0, 0),
        RGB {
            r: 255,
            g: 18,
            b: 0
        }
    );
    assert_eq!(o.get(1, 0), RGB { r: 0, g: 182, b: 0 });
    assert_eq!(
        o.get(2, 0),
        RGB {
            r: 0,
            g: 131,
            b: 255
        }
    );
    assert_eq!(o.get(0, 1), RGB { r: 0, g: 182, b: 0 });
    assert_eq!(
        o.get(1, 1),
        RGB {
            r: 0,
            g: 121,
            b: 255
        }
    );
    assert_eq!(
        o.get(2, 1),
        RGB {
            r: 255,
            g: 18,
            b: 0
        }
    );
    assert_eq!(
        o.get(0, 2),
        RGB {
            r: 0,
            g: 131,
            b: 255
        }
    );
    assert_eq!(
        o.get(1, 2),
        RGB {
            r: 255,
            g: 18,
            b: 0
        }
    );
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

    assert_eq!(
        o.get(0, 0),
        RGB {
            r: 255,
            g: 0,
            b: 255
        }
    );
    assert_eq!(
        o.get(0, 1),
        RGB {
            r: 0,
            g: 255,
            b: 255
        }
    );
    assert_eq!(o.get(0, 2), RGB { r: 0, g: 0, b: 255 });
    assert_eq!(
        o.get(1, 0),
        RGB {
            r: 255,
            g: 255,
            b: 0
        }
    );
    assert_eq!(
        o.get(1, 1),
        RGB {
            r: 255,
            g: 200,
            b: 0
        }
    );
    assert_eq!(o.get(1, 2), RGB { r: 0, g: 0, b: 0 });
    assert_eq!(o.get(2, 0), RGB { r: 0, g: 0, b: 255 });
    assert_eq!(
        o.get(2, 1),
        RGB {
            r: 255,
            g: 0,
            b: 255
        }
    );
    assert_eq!(
        o.get(2, 2),
        RGB {
            r: 0,
            g: 255,
            b: 255
        }
    );
}

#[test]
fn analyze_test() {
    let image = TestImage::new_from_fn(24, 24, |x, y| {
        if x >= 8 && x < 16 && y >= 8 && y < 16 {
            SKIN
        } else {
            WHITE
        }
    });

    let crop = analyse(
        &CropSettings::default(),
        &image,
        NonZeroU32::new(8).unwrap(),
        NonZeroU32::new(8).unwrap(),
        1.0,
    );

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
