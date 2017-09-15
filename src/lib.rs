mod math;

use self::math::*;

use std::fmt;

const prescale: bool = true;
const prescaleMin: f64 = 400.00;
const minScale: f64 = 1.0;
const maxScale: f64 = 1.0;
const step: f64 = 8.0;
const scaleStep: f64 = 0.1;

const scoreDownSample: f64 = 8.0;

const saturationBias: f64 = 0.2;
const skinBias: f64 = 0.01;

const skinWeight: f64 = 1.8;
const saturationWeight: f64 = 0.3;
const detailWeight: f64 = 0.2;

const skinBrightnessMin: f64 = 0.2;
const skinBrightnessMax: f64 = 1.0;
const skinThreshold: f64 = 0.8;

const saturationBrightnessMin: f64 = 0.05;
const saturationBrightnessMax: f64 = 0.9;
const saturationThreshold: f64 = 0.4;
// step * minscale rounded down to the next power of two should be good
// step * minscale rounded down to the next power of two should be good
const edgeRadius: f64 = 0.4;
const edgeWeight: f64 = -20.0;
const outsideImportance: f64 = -0.5;
const ruleOfThirds: bool = true;

//TODO Check all `as uXX` casts. Should be rounded first

trait Image {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn resize(&self, width: u32) -> Box<Image>;
    fn get(&self, x: u32, y: u32) -> RGB;
}

struct ImageMap {
    width: u32,
    height: u32,

    pixels: Vec<Vec<RGB>>
}

impl ImageMap {
    fn new(width: u32, height: u32) -> ImageMap {
        let white = RGB::new(255, 255, 255);
        let pixels = vec![vec![white; height as usize]; width as usize];
        ImageMap {
            width,
            height,
            pixels
        }
    }

    fn set(&mut self, x: u32, y: u32, color: RGB) {
        self.pixels[x as usize][y as usize] = color
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        self.pixels[x as usize][y as usize]
    }

    fn downSample(&self, factor: u32) -> Self {
        //        let idata = self.data;
        let iwidth = self.width;
        let width = (self.width as f64 / factor as f64) as u32;
        let height = (self.height as f64 / factor as f64) as u32;
        let mut output = ImageMap::new(width, height);
        //        let data = output.data;
        let ifactor2: f64 = 1.0 / (factor as f64 * factor as f64);

        let max = |a, b| {
            if a > b {
                a
            } else {
                b
            }
        };

        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 4;

                let mut r: f64 = 0.0;
                let mut g: f64 = 0.0;
                let mut b: f64 = 0.0;
                let a = 0;

                let mut mr: f64 = 0.0;
                let mut mg: f64 = 0.0;
                let mut mb: f64 = 0.0;

                for v in 0..factor as u32 {
                    for u in 0..factor {
                        let ix = x * factor + u;
                        let iy = y * factor + v;
                        let icolor = self.get(ix, iy);

                        r += icolor.r as f64;
                        g += icolor.g as f64;
                        b += icolor.b as f64;
                        mr = max(mr, icolor.r as f64);
                        mg = max(mg, icolor.g as f64);
                        mb = max(mb, icolor.b as f64);
                    }
                }

                // this is some funky magic to preserve detail a bit more for
                // skin (r) and detail (g). Saturation (b) does not get this boost.
                output.set(x, y, RGB::new(
                    (r * ifactor2 * 0.5 + mr * 0.5).round() as u8,
                    (g * ifactor2 * 0.7 + mg * 0.3).round() as u8,
                    (b * ifactor2).round() as u8)
                )
            }
        }

        output
    }
}

trait Analyzer {
    fn find_best_crop(&self, image: &Image, width: u32, height: u32) -> Result<ScoredCrop, String>;
}

struct CropSettings {}

impl CropSettings {
    fn default() -> CropSettings {
        CropSettings {}
    }
}

struct BasicAnalyzer {
    settings: CropSettings
}

impl BasicAnalyzer {
    fn new(settings: CropSettings) -> BasicAnalyzer {
        BasicAnalyzer { settings }
    }
}

impl Analyzer for BasicAnalyzer {
    fn find_best_crop(&self, img: &Image, width: u32, height: u32) -> Result<ScoredCrop, String> {
        if (width == 0 && height == 0) {
            return Err("Expect either a height or width".to_owned());
        }
        let width = width as f64;
        let height = height as f64;

        let scale = f64::min((img.width() as f64) / width, (img.height() as f64) / height);

        // resize image for faster processing
        let prescalefactor = 1.0;

        let cropWidth = chop(width * scale * prescalefactor) as u32;
        let cropHeight = chop(height * scale * prescalefactor) as u32;
        let realMinScale = f64::min(maxScale, f64::max(1.0 / scale, minScale));

        if prescale {
            let f = prescaleMin / f64::min((img.width() as f64), (img.height() as f64));
            let prescalefactor = if f < 1.0 {
                f
            } else {
                prescalefactor
            };

            let resize_result = img.resize(((img.width() as f64) * prescalefactor) as u32);

            let img = resize_result.as_ref();

            let topCrop = try!(analyse(&self.settings, img, cropWidth, cropHeight, realMinScale)).unwrap();

            Ok(topCrop.scale(1.0 / prescalefactor))
        } else {
            let topCrop = try!(analyse(&self.settings, img, cropWidth, cropHeight, realMinScale));
            //TODO check
            Ok(topCrop.unwrap())
        }
    }
}

fn analyse(cs: &CropSettings, img: &Image, cropWidth: u32, cropHeight: u32, realMinScale: f64) -> Result<Option<ScoredCrop>, String> {
    let mut o = ImageMap::new(img.width(), img.height());

    edgeDetect(img, &mut o);

    skinDetect(img, &mut o);

    saturationDetect(img, &mut o);

    //TODO check if crops can return empty vector
    let cs: Vec<Crop> = crops(&o, cropWidth, cropHeight, realMinScale);
    let scoreOutput = o.downSample(scoreDownSample as u32);
    let topCrop: Option<ScoredCrop> = cs.iter()
                                        .map(|crop| {
                                            let crop = ScoredCrop { crop: crop.clone(), score: score(&scoreOutput, &crop) };
                                            crop
                                        })
                                        .fold(None, |result, scoredCrop| {
                                            Some(match result {
                                                None => scoredCrop,
                                                Some(result) => if (result.score.Total > scoredCrop.score.Total) {
                                                    result
                                                } else {
                                                    scoredCrop
                                                }
                                            })
                                        });

    Ok(topCrop)
}

fn edgeDetect(i: &Image, o: &mut ImageMap) {
    //TODO check type casts if those are safe

    let w = i.width() as usize;
    let h = i.height() as usize;
    let cies = makeCies(i);

    for y in 0..h {
        for x in 0..w {
            let color = i.get(x as u32, y as u32);

            let lightness = if (x == 0 || x >= w - 1 || y == 0 || y >= h - 1) {
                cies[y * w + x]
            } else {
                cies[y * w + x] * 4.0 -
                    cies[x + (y - 1) * w] -
                    cies[x - 1 + y * w] -
                    cies[x + 1 + y * w] -
                    cies[x + (y + 1) * w]
            };

            let g = bounds(lightness);

            let nc = RGB { g: g, ..color };
            o.set(x as u32, y as u32, nc)
        }
    }
}

fn makeCies(img: &Image) -> Vec<f64> {
    //TODO `cies()` can probably be made RGB member that will make this function redundant
    let w = img.width();
    let h = img.height();
    let size = (w as u64 * h as u64);

    let size = if size > usize::max_value() as u64 {
        None
    } else {
        Some(size as usize)
    };

    //TODO error handling
    let mut cies = Vec::with_capacity(size.expect("Too big image dimensions"));

    let mut i: usize = 0;
    for y in 0..h {
        for x in 0..w {
            let color = img.get(x, y);
            cies.insert(i, cie(color));
            i += 1;
        };
    };

    cies
}

fn crops(i: &ImageMap, cropWidth: u32, cropHeight: u32, realMinScale: f64) -> Vec<Crop> {
    let mut crops: Vec<Crop> = vec![];
    let width = i.width as f64;
    let height = i.height as f64;

    let minDimension = f64::min(width, height);

    let cropW = if cropWidth != 0 { cropWidth as f64 } else { minDimension };
    let cropH = if cropHeight != 0 { cropHeight as f64 } else { minDimension };

    let mut scale = maxScale;
    loop {
        if (scale < realMinScale) {
            break;
        }

        for y in (0..).map(|i: u32| i as f64 * step)
                      .take_while(|y| y + cropH * scale <= height) {
            for x in (0..).map(|i: u32| i as f64 * step)
                          .take_while(|x| x + cropW * scale <= width) {
                crops.push(Crop {
                    X: x as u32,
                    Y: y as u32,
                    Width: (cropW * scale) as u32,
                    Height: (cropH * scale) as u32,

                });
            }
        }

        scale -= scaleStep;
    }

    crops
}

fn score(o: &ImageMap, crop: &Crop) -> Score {
    let height = o.height as f64;
    let width = o.width as f64;

    let downSample = scoreDownSample;
    let invDownSample = 1.0 / downSample;
    let outputHeightDownSample = height * downSample;
    let outputWidthDownSample = width * downSample;
    let outputWidth = width;

    let mut Skin = 0.0;
    let mut Detail = 0.0;
    let mut Saturation = 0.0;

    for y in (0..).map(|i: u32| i as f64 * scoreDownSample)
                  .take_while(|&y| y < outputHeightDownSample) {
        for x in (0..).map(|i: u32| i as f64 * scoreDownSample)
                      .take_while(|&x| x < outputWidthDownSample) {
            let orig_x = (x * invDownSample) as u32;
            let orig_y = (y * invDownSample) as u32;

            let color = o.get(orig_x, orig_y);

            let imp = importance(crop, x as u32, y as u32);
            let det = color.g as f64 / 255.0;

            Skin += color.r as f64 / 255.0 * (det + skinBias) * imp;
            Detail += det * imp;
            Saturation += color.b as f64 / 255.0 * (det + saturationBias) * imp;
        };
    };

    let Total = (Detail * detailWeight + Skin * skinWeight + Saturation * saturationWeight) / crop.Width as f64 / crop.Height as f64;

    Score {
        Skin,
        Detail,
        Saturation,
        Total
    }
}

fn skinDetect(i: &Image, o: &mut ImageMap) {
    let w = i.width();
    let h = i.height();

    for y in (0..h) {
        for x in (0..w) {
            let lightness = cie(i.get(x, y)) / 255.0;
            let skin = skinCol(i.get(x, y));

            let nc = if skin > skinThreshold && lightness >= skinBrightnessMin && lightness <= skinBrightnessMax {
                let r = (skin - skinThreshold) * (255.0 / (1.0 - skinThreshold));
                let RGB { r: _, g: g, b: b } = o.get(x, y);

                RGB { r: bounds(r), g, b }
            } else {
                let RGB { r: _, g: g, b: b } = o.get(x, y);
                RGB { r: 0, g, b }
            };

            o.set(x, y, nc);
        }
    }
}

fn saturationDetect(i: &Image, o: &mut ImageMap) {
    let w = i.width();
    let h = i.height();

    for y in (0..h) {
        for x in (0..w) {
            let color = i.get(x, y);
            let lightness = cie(color) / 255.0;
            let saturation = saturation(color);

            let nc = if saturation > saturationThreshold
                && lightness >= saturationBrightnessMin
                && lightness <= saturationBrightnessMax {
                let b = (saturation - saturationThreshold) * (255.0 / (1.0 - saturationThreshold));
                let RGB { r: r, g: g, b: _ } = o.get(x, y);
                RGB { r, g, b: bounds(b) }
            } else {
                let RGB { r: r, g: g, b: _ } = o.get(x, y);
                RGB { r, g, b: 0 }
            };

            o.set(x, y, nc);
        }
    }
}

#[cfg(test)]
mod tests {
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
            if (x != 0 || y != 0) {
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

            for y in (0..image.height()) {
                for x in (0..image.width()) {
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
            println!("{:?}", width);
            if width == self.w {
                return Box::new(self.clone())
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
    fn ImageMap_test() {
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
        let realMinScale = minScale;

        let crops = crops(&ImageMap::new(8, 8), 8, 8, realMinScale);

        assert_eq!(crops[0], Crop { X: 0, Y: 0, Width: 8, Height: 8 })
    }

    #[test]
    fn score__image_with_single_black_pixel__score_is_zero() {
        let mut i = ImageMap::new(1, 1);
        i.set(0, 0, RGB::new(0, 0, 0));

        let s = score(&i, &Crop { X: 0, Y: 0, Width: 1, Height: 1 });

        assert_eq!(s, Score { Detail: 0.0, Saturation: 0.0, Skin: 0.0, Total: 0.0 });
    }

    #[test]
    fn score__image_with_single_white_pixel__score_is_the_same_as_for_js_version() {
        let mut i = ImageMap::new(1, 1);
        i.set(0, 0, RGB::new(255, 255, 255));

        let s = score(&i, &Crop { X: 0, Y: 0, Width: 1, Height: 1 });

        let js_version_score = Score {
            Detail: -6.404213562373096,
            Saturation: -7.685056274847715,
            Skin: -6.468255697996827,
            Total: -15.229219851323222
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
    fn skinDetect_single_pixel_test() {
        let detect_pixel = |color: RGB| {
            let image = SinglePixelImage::new(color);
            let mut o = ImageMap::new(1, 1);
            o.set(0, 0, color);

            skinDetect(&image, &mut o);
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
    fn edgeDetect_single_pixel_image_test() {
        let edgeDetect_pixel = |color: RGB| {
            let image = SinglePixelImage::new(color);
            let mut o = ImageMap::new(1, 1);
            o.set(0, 0, color);

            edgeDetect(&image, &mut o);

            o.get(0, 0)
        };

        assert_eq!(edgeDetect_pixel(BLACK), BLACK);
        assert_eq!(edgeDetect_pixel(WHITE), WHITE);
        assert_eq!(edgeDetect_pixel(RED).round(), RGB::new(255, 18, 0));
        assert_eq!(edgeDetect_pixel(GREEN).round(), RGB::new(0, 182, 0));
        assert_eq!(edgeDetect_pixel(BLUE).round(), RGB::new(0, 131, 255));
        assert_eq!(edgeDetect_pixel(SKIN).round(), RGB::new(255, 243, 159));
    }

    //#[test]
    fn edgeDetect_3x3() {
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

        edgeDetect(&image, &mut o);

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
    fn saturationDetect_3x3() {
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

        saturationDetect(&image, &mut o);

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

        assert_eq!(crop.crop.Width, 8);
        assert_eq!(crop.crop.Height, 8);
        assert_eq!(crop.crop.X, 8);
        assert_eq!(crop.crop.Y, 8);
        assert_eq!(crop.score.Saturation, 0.0);
        assert_eq!(crop.score.Detail, -1.7647058823529413);
        assert_eq!(crop.score.Skin, -0.03993215515362048);
        assert_eq!(crop.score.Total, -0.006637797746048519);
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
        let analyzer = BasicAnalyzer::new(CropSettings::default());

        let crop = analyzer.find_best_crop(&image, 8, 8).unwrap();

        assert_eq!(crop.crop.Width, 8);
        assert_eq!(crop.crop.Height, 8);
        assert_eq!(crop.crop.Y, 0);
        assert_eq!(crop.crop.X, 16);
        assert_eq!(crop.score.Detail, -4.040026482281278);
        assert_eq!(crop.score.Saturation, -0.3337408688965783);
        assert_eq!(crop.score.Skin, -0.13811572472126107);
        assert_eq!(crop.score.Total, -0.018073997837867173);
    }

    #[test]
    fn downSample_test() {
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

        let result = image_map.downSample(3);

        assert_eq!(result.width, 1);
        assert_eq!(result.height, 1);
        assert_eq!(result.get(0, 0), RGB::new(184, 132, 103));
    }
}