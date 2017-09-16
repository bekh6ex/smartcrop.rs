mod math;

use self::math::*;

use std::fmt;

const PRESCALE: bool = true;
const PRESCALE_MIN: f64 = 400.00;
const MIN_SCALE: f64 = 1.0;
const MAX_SCALE: f64 = 1.0;
const STEP: f64 = 8.0;
const SCALE_STEP: f64 = 0.1;

const SCORE_DOWN_SAMPLE: f64 = 8.0;

const SATURATION_BIAS: f64 = 0.2;
const SKIN_BIAS: f64 = 0.01;

const SKIN_WEIGHT: f64 = 1.8;
const SATURATION_WEIGHT: f64 = 0.3;
const DETAIL_WEIGHT: f64 = 0.2;

const SKIN_BRIGHTNESS_MIN: f64 = 0.2;
const SKIN_BRIGHTNESS_MAX: f64 = 1.0;
const SKIN_THRESHOLD: f64 = 0.8;

const SATURATION_BRIGHTNESS_MIN: f64 = 0.05;
const SATURATION_BRIGHTNESS_MAX: f64 = 0.9;
const SATURATION_THRESHOLD: f64 = 0.4;
// STEP * minscale rounded down to the next power of two should be good
// STEP * minscale rounded down to the next power of two should be good
const EDGE_RADIUS: f64 = 0.4;
const EDGE_WEIGHT: f64 = -20.0;
const OUTSIDE_IMPORTANCE: f64 = -0.5;
const RULE_OF_THIRDS: bool = true;

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

    fn down_sample(&self, factor: u32) -> Self {
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
                // skin (r) and detail (g). saturation (b) does not get this boost.
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
        if width == 0 && height == 0 {
            return Err("Expect either a height or width".to_owned());
        }
        let width = width as f64;
        let height = height as f64;

        let scale = f64::min((img.width() as f64) / width, (img.height() as f64) / height);

        // resize image for faster processing
        let prescalefactor = 1.0;

        let crop_width = chop(width * scale * prescalefactor) as u32;
        let crop_height = chop(height * scale * prescalefactor) as u32;
        let real_min_scale = f64::min(MAX_SCALE, f64::max(1.0 / scale, MIN_SCALE));

        if PRESCALE {
            let f = PRESCALE_MIN / f64::min((img.width() as f64), (img.height() as f64));
            let prescalefactor = if f < 1.0 {
                f
            } else {
                prescalefactor
            };

            let resize_result = img.resize(((img.width() as f64) * prescalefactor) as u32);

            let img = resize_result.as_ref();

            let top_crop = try!(analyse(&self.settings, img, crop_width, crop_height, real_min_scale)).unwrap();

            Ok(top_crop.scale(1.0 / prescalefactor))
        } else {
            let top_crop = try!(analyse(&self.settings, img, crop_width, crop_height, real_min_scale));
            //TODO check
            Ok(top_crop.unwrap())
        }
    }
}

fn analyse(cs: &CropSettings, img: &Image, crop_width: u32, crop_height: u32, real_min_scale: f64) -> Result<Option<ScoredCrop>, String> {
    let mut o = ImageMap::new(img.width(), img.height());

    edge_detect(img, &mut o);

    skin_detect(img, &mut o);

    saturation_detect(img, &mut o);

    //TODO check if crops can return empty vector
    let cs: Vec<Crop> = crops(&o, crop_width, crop_height, real_min_scale);
    let score_output = o.down_sample(SCORE_DOWN_SAMPLE as u32);
    let top_crop: Option<ScoredCrop> = cs.iter()
                                         .map(|crop| {
                                            let crop = ScoredCrop { crop: crop.clone(), score: score(&score_output, &crop) };
                                            crop
                                        })
                                         .fold(None, |result, scored_crop| {
                                            Some(match result {
                                                None => scored_crop,
                                                Some(result) => if result.score.total > scored_crop.score.total {
                                                    result
                                                } else {
                                                    scored_crop
                                                }
                                            })
                                        });

    Ok(top_crop)
}

fn edge_detect(i: &Image, o: &mut ImageMap) {
    //TODO check type casts if those are safe

    let w = i.width() as usize;
    let h = i.height() as usize;
    let cies = make_cies(i);

    for y in 0..h {
        for x in 0..w {
            let color = i.get(x as u32, y as u32);

            let lightness = if x == 0 || x >= w - 1 || y == 0 || y >= h - 1 {
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

fn make_cies(img: &Image) -> Vec<f64> {
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

fn crops(i: &ImageMap, crop_width: u32, crop_height: u32, real_min_scale: f64) -> Vec<Crop> {
    let mut crops: Vec<Crop> = vec![];
    let width = i.width as f64;
    let height = i.height as f64;

    let min_dimension = f64::min(width, height);

    let crop_w = if crop_width != 0 { crop_width as f64 } else { min_dimension };
    let crop_h = if crop_height != 0 { crop_height as f64 } else { min_dimension };

    let mut scale = MAX_SCALE;
    loop {
        if scale < real_min_scale {
            break;
        }

        for y in (0..).map(|i: u32| i as f64 * STEP)
                      .take_while(|y| y + crop_h * scale <= height) {
            for x in (0..).map(|i: u32| i as f64 * STEP)
                          .take_while(|x| x + crop_w * scale <= width) {
                crops.push(Crop {
                    x: x as u32,
                    y: y as u32,
                    width: (crop_w * scale) as u32,
                    height: (crop_h * scale) as u32,

                });
            }
        }

        scale -= SCALE_STEP;
    }

    crops
}

fn score(o: &ImageMap, crop: &Crop) -> Score {
    let height = o.height as f64;
    let width = o.width as f64;

    let down_sample = SCORE_DOWN_SAMPLE;
    let inv_down_sample = 1.0 / down_sample;
    let output_height_down_sample = height * down_sample;
    let output_width_down_sample = width * down_sample;
    let output_width = width;

    let mut skin = 0.0;
    let mut detail = 0.0;
    let mut saturation = 0.0;

    for y in (0..).map(|i: u32| i as f64 * SCORE_DOWN_SAMPLE)
                  .take_while(|&y| y < output_height_down_sample) {
        for x in (0..).map(|i: u32| i as f64 * SCORE_DOWN_SAMPLE)
                      .take_while(|&x| x < output_width_down_sample) {
            let orig_x = (x * inv_down_sample) as u32;
            let orig_y = (y * inv_down_sample) as u32;

            let color = o.get(orig_x, orig_y);

            let imp = importance(crop, x as u32, y as u32);
            let det = color.g as f64 / 255.0;

            skin += color.r as f64 / 255.0 * (det + SKIN_BIAS) * imp;
            detail += det * imp;
            saturation += color.b as f64 / 255.0 * (det + SATURATION_BIAS) * imp;
        };
    };

    let total = (detail * DETAIL_WEIGHT + skin * SKIN_WEIGHT + saturation * SATURATION_WEIGHT) / crop.width as f64 / crop.height as f64;

    Score {
        skin: skin,
        detail: detail,
        saturation: saturation,
        total: total
    }
}

fn skin_detect(i: &Image, o: &mut ImageMap) {
    let w = i.width();
    let h = i.height();

    for y in 0..h {
        for x in 0..w {
            let lightness = cie(i.get(x, y)) / 255.0;
            let skin = skin_col(i.get(x, y));

            let nc = if skin > SKIN_THRESHOLD && lightness >= SKIN_BRIGHTNESS_MIN && lightness <= SKIN_BRIGHTNESS_MAX {
                let r = (skin - SKIN_THRESHOLD) * (255.0 / (1.0 - SKIN_THRESHOLD));
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

fn saturation_detect(i: &Image, o: &mut ImageMap) {
    let w = i.width();
    let h = i.height();

    for y in 0..h {
        for x in 0..w {
            let color = i.get(x, y);
            let lightness = cie(color) / 255.0;
            let saturation = saturation(color);

            let nc = if saturation > SATURATION_THRESHOLD
                && lightness >= SATURATION_BRIGHTNESS_MIN
                && lightness <= SATURATION_BRIGHTNESS_MAX {
                let b = (saturation - SATURATION_THRESHOLD) * (255.0 / (1.0 - SATURATION_THRESHOLD));
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
mod tests;