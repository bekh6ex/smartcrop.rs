use super::*;


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

impl SinglePixelImage {
    fn new(pixel: RGB) -> SinglePixelImage {
        SinglePixelImage { pixel }
    }
}

impl Image for SinglePixelImage {
    fn width(&self) -> u32 {
        1
    }

    fn height(&self) -> u32 {
        1
    }

    fn resize(&self, width: u32) -> Box<Image> {
        unimplemented!()
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        if x != 0 || y != 0 {
            panic!("Index overflow. x: {}, y: {}", x, y);
        }

        self.pixel
    }
}

#[derive(Debug, Clone)]
struct TestImage {
    w: u32,
    h: u32,
    pixels: Vec<Vec<RGB>>
}

impl TestImage {
    fn new(w: u32, h: u32, pixels: Vec<Vec<RGB>>) -> TestImage {
        TestImage { w, h, pixels }
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
}

impl ImageMap {
    fn from_image(image: &Image) -> ImageMap {
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

//    impl<'a> From<&'a Image> for ImageMap {
//
//    }

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
fn score__image_with_single_black_pixel__score_is_zero() {
    let mut i = ImageMap::new(1, 1);
    i.set(0, 0, RGB::new(0, 0, 0));

    let s = score(&i, &Crop { x: 0, y: 0, width: 1, height: 1 });

    assert_eq!(s, Score { detail: 0.0, saturation: 0.0, skin: 0.0, total: 0.0 });
}

#[test]
fn score__image_with_single_white_pixel__score_is_the_same_as_for_js_version() {
    let mut i = ImageMap::new(1, 1);
    i.set(0, 0, RGB::new(255, 255, 255));

    let s = score(&i, &Crop { x: 0, y: 0, width: 1, height: 1 });

    let js_version_score = Score {
        detail: -6.404213562373096,
        saturation: -7.685056274847715,
        skin: -6.468255697996827,
        total: -15.229219851323222
    };

    assert_eq!(s, js_version_score);
}

impl RGB {
    fn round(&self) -> RGB {
        //TODO Probably should be removed
        RGB { r: self.r, g: self.g, b: self.b }
    }
}

//#[test]
fn skin_detect_single_pixel_test() {
    let detect_pixel = |color: RGB| {
        let image = SinglePixelImage::new(color);
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
    assert_eq!(detect_pixel(SKIN).round(), RGB::new(159, 200, 159));
}

//#[test]
fn edge_detect_single_pixel_image_test() {
    let edge_detect_pixel = |color: RGB| {
        let image = SinglePixelImage::new(color);
        let mut o = ImageMap::new(1, 1);
        o.set(0, 0, color);

        edge_detect(&image, &mut o);

        o.get(0, 0)
    };

    assert_eq!(edge_detect_pixel(BLACK), BLACK);
    assert_eq!(edge_detect_pixel(WHITE), WHITE);
    assert_eq!(edge_detect_pixel(RED).round(), RGB::new(255, 18, 0));
    assert_eq!(edge_detect_pixel(GREEN).round(), RGB::new(0, 182, 0));
    assert_eq!(edge_detect_pixel(BLUE).round(), RGB::new(0, 131, 255));
    assert_eq!(edge_detect_pixel(SKIN).round(), RGB::new(255, 243, 159));
}

//#[test]
fn edge_detect_3x3() {
    let image = TestImage::new(
        3,
        3,
        vec![
            vec![RED, GREEN, BLUE],
            vec![GREEN, BLUE, RED],
            vec![BLUE, RED, GREEN],
        ]
    );
    let mut o = ImageMap::new(3, 3);

    edge_detect(&image, &mut o);

    assert_eq!(o.get(0, 0), RGB { r: 255, g: 18, b: 0 });
    assert_eq!(o.get(0, 0), RGB { r: 255, g: 18, b: 0 });
    assert_eq!(o.get(1, 0), RGB { r: 0, g: 182, b: 0 });
    assert_eq!(o.get(2, 0), RGB { r: 0, g: 130, b: 255 });
    assert_eq!(o.get(0, 1), RGB { r: 0, g: 182, b: 0 });
    assert_eq!(o.get(1, 1), RGB { r: 0, g: 121, b: 255 });
    assert_eq!(o.get(2, 1), RGB { r: 255, g: 18, b: 0 });
    assert_eq!(o.get(0, 2), RGB { r: 0, g: 130, b: 255 });
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
        ]
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
        }
    );

    let crop = analyse(&CropSettings::default(), &image, 8, 8, 1.0).unwrap().unwrap();

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

    let crop = analyzer.find_best_crop(&image, 8, 8).unwrap();

    assert_eq!(crop.crop.width, 8);
    assert_eq!(crop.crop.height, 8);
    assert_eq!(crop.crop.y, 0);
    assert_eq!(crop.crop.x, 16);
    assert_eq!(crop.score.detail, -4.040026482281278);
    assert_eq!(crop.score.saturation, -0.3337408688965783);
    assert_eq!(crop.score.skin, -0.13811572472126107);
    assert_eq!(crop.score.total, -0.018073997837867173);
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
        ]
    );

    let image_map = ImageMap::from_image(&image);

    let result = image_map.down_sample(3);

    assert_eq!(result.width, 1);
    assert_eq!(result.height, 1);
    assert_eq!(result.get(0, 0), RGB::new(184, 132, 103));
}
