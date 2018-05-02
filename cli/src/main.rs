extern crate smartcrop;

use smartcrop::{CropSettings, Analyzer, RGB};

extern crate clap;

use clap::{Arg, App, SubCommand};

extern crate image;

use image::{GenericImage, ImageBuffer, DynamicImage, FilterType};

fn main() {
    let an: Analyzer = Analyzer::new(CropSettings::default());

    let matches = App::new("asd")
        .arg(Arg::with_name("INPUT").required(true))
        .arg(Arg::with_name("OUTPUT").required(true))
        .get_matches();

    let file_in = matches.value_of("INPUT").unwrap();
    let file_out = matches.value_of("OUTPUT").unwrap();

    // Use the open function to load an image from a Path.
    // ```open``` returns a `DynamicImage` on success.
    let mut img = image::open(file_in).unwrap();

    let crop = an.find_best_crop(&SmartCropImage{image:img.clone()}, 10, 10).unwrap().crop;

    let cropped = img.crop(crop.x, crop.y, crop.width, crop.height);

    cropped.save(file_out).unwrap();

    println!("{:?}", crop)
}

#[derive(Clone)]
struct SmartCropImage { image: DynamicImage }

impl smartcrop::Image for SmartCropImage {
    fn width(&self) -> u32 {
        self.image.dimensions().0
    }

    fn height(&self) -> u32 {
        self.image.dimensions().1
    }

    fn resize(&self, width: u32) -> Box<smartcrop::Image> {
        if width == self.width() {
            return Box::new(self.clone());
        }

        let resized = self.image.resize(width, self.height(), FilterType::Lanczos3);

        Box::new(SmartCropImage{image:resized})
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        let px = self.image.get_pixel(x, y);
        let r = px[0];
        let g = px[1];
        let b = px[2];
        RGB{r,g,b}
    }
}