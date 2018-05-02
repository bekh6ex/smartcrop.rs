extern crate image as image_ext;

use self::image_ext::{DynamicImage, FilterType, GenericImage};
use RGB;
use Image;

impl Image for DynamicImage {
    type ResizeToImage = Self;

    fn width(&self) -> u32 {
        GenericImage::width(self)
    }

    fn height(&self) -> u32 {
        GenericImage::height(self)
    }

    fn resize(&self, width: u32) -> Self {
        if width == GenericImage::width(self) {
            return self.clone();
        }
        let height = GenericImage::height(self);

        self.resize(width, height, FilterType::Lanczos3)
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        let px = self.get_pixel(x, y);
        let r = px[0];
        let g = px[1];
        let b = px[2];
        RGB{r,g,b}
    }
}