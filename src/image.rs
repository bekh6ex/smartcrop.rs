extern crate image as image_ext;

use self::image_ext::{FilterType, GenericImage, Pixel, imageops};
use RGB;
use Image;
use std;

impl<I, P> Image for I
    where I: GenericImage<Pixel=P> + Clone + 'static,
          P: Pixel<Subpixel=u8> + 'static {
    type ResizeToImage = self::image_ext::ImageBuffer<P, std::vec::Vec<P::Subpixel>>;


    fn width(&self) -> u32 {
        GenericImage::width(self)
    }

    fn height(&self) -> u32 {
        GenericImage::height(self)
    }

    fn resize(&self, width: u32) -> Self::ResizeToImage {
        let height = (width as f64 / GenericImage::width(self) as f64 * GenericImage::height(self) as f64).round() as u32;

        imageops::resize(self, width, height, FilterType::Lanczos3)
    }

    fn get(&self, x: u32, y: u32) -> RGB {
        let px = self.get_pixel(x, y).to_rgb();

        let r = px[0];
        let g = px[1];
        let b = px[2];
        RGB{r,g,b}
    }
}
