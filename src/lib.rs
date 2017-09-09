mod math;

use self::math::*;

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
}

trait Analyzer {
    fn find_best_crop(&self, image: &Image, width: u32, height: u32) -> Result<ScoredCrop, String>;
}

struct CropSettings {}

struct BasicAnalyzer {
    crop_settings: CropSettings
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

            let topCrop = try!(analyse(&self.crop_settings, img, cropWidth, cropHeight, realMinScale)).unwrap();

            Ok(topCrop.scale(1.0 / prescalefactor))
        } else {
            let topCrop = try!(analyse(&self.crop_settings, img, cropWidth, cropHeight, realMinScale));
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
    let topCrop: Option<ScoredCrop> = cs.iter()
                                        .map(|crop| ScoredCrop { crop: crop.clone(), score: score(&o, &crop) })
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
            let lightness = if (x == 0 || x >= w - 1 || y == 0 || y >= h - 1) {
                //lightness = cie((*i).At(x, y))
                0.0
            } else {
                cies[y * w + x] * 4.0 -
                    cies[x + (y - 1) * w] -
                    cies[x - 1 + y * w] -
                    cies[x + 1 + y * w] -
                    cies[x + (y + 1) * w]
            };

            let g = bounds(lightness) as u8;

            let nc = RGB::new(0, g, 0);
            o.set(x as u32, y as u32, nc)
        }
    }
}

fn makeCies(img: &Image) -> Vec<f64> {
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
            cies[i] = cie(color);
            i += 1;
        }
    }

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

    let mut Skin = 0.0;
    let mut Detail = 0.0;
    let mut Saturation = 0.0;

    for y in (0..).map(|i: u32| i as f64 * scoreDownSample)
                  .take_while(|y| y < &(height * scoreDownSample)) {
        for x in (0..).map(|i: u32| i as f64 * scoreDownSample)
                      .take_while(|x| x < &(width * scoreDownSample)) {
            let color = o.get(x as u32, y as u32);

            let imp = importance(crop, x as u32, y as u32);
            let det = color.g / 255.0;

            Skin += color.r / 255.0 * (det + skinBias) * imp;
            Detail += det * imp;
            Saturation += color.b / 255.0 * (det + saturationBias) * imp;
        }
    }

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
                RGB { r: 0.0, g, b }
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
            let lightness = cie(i.get(x, y)) / 255.0;
            let skin = skinCol(i.get(x, y));

            let nc = if skin > skinThreshold && lightness >= skinBrightnessMin && lightness <= skinBrightnessMax {
                let r = (skin - skinThreshold) * (255.0 / (1.0 - skinThreshold));
                let RGB { r: _, g: g, b: b } = o.get(x, y);

                RGB { r: bounds(r), g, b }
            } else {
                let RGB { r: _, g: g, b: b } = o.get(x, y);
                RGB { r: 0.0, g, b }
            };

            o.set(x, y, nc);
        }
    }

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
                RGB { r, g, b: 0.0 }
            };

            o.set(x, y, nc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}