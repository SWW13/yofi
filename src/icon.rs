use std::io::BufReader;
use std::path::Path;

use anyhow::{anyhow, Context, Result};

pub struct Icon {
    width: u32,
    height: u32,
    data: Vec<u32>,
}

impl Icon {
    pub fn load_icon(path: impl AsRef<Path>) -> Option<Icon> {
        let path = path.as_ref();
        let failed_to_load = |e| log::info!("failed to load icon at path `{:?}`: {}", path, e);
        match path.extension()?.to_str()? {
            "png" => Icon::from_png_path(path).map_err(failed_to_load).ok(),
            "svg" => Icon::from_svg_path(path).map_err(failed_to_load).ok(),
            ext => {
                log::info!("unsupported icon extension: {:?}", ext);
                None
            }
        }
    }

    fn from_png_path(path: impl AsRef<Path>) -> Result<Self> {
        let decoder = png::Decoder::new(BufReader::new(std::fs::File::open(path)?));
        let mut reader = decoder
            .read_info()
            .map_err(|e| anyhow!("failed to read png info: {}", e))?;
        let mut buf = vec![0; reader.output_buffer_size()];
        reader
            .next_frame(&mut buf)
            .map_err(|e| anyhow!("failed to read png frame: {}", e))?;

        let info = reader.info();
        let data = match info.color_type {
            png::ColorType::Rgb => {
                let mut data = vec![];

                for chunk in buf.chunks(3) {
                    let a = 0xffu32 << 24;
                    let r = u32::from(chunk[0]) << 16;
                    let g = u32::from(chunk[1]) << 8;
                    let b = u32::from(chunk[2]);

                    data.push(a | r | g | b);
                }

                data
            }
            png::ColorType::Rgba => rgba_to_argb(buf.as_slice()),
            color_type => anyhow::bail!("unsupported icon color type {:?}", color_type),
        };

        Ok(Self {
            width: info.width,
            height: info.height,
            data,
        })
    }

    fn from_svg_path(path: impl AsRef<Path>) -> Result<Self> {
        let opt = usvg::Options::default();
        let data = std::fs::read(path.as_ref())
            .with_context(|| format!("failed to open svg file: {:?}", path.as_ref()))?;
        let tree = usvg::Tree::from_data(&data, &opt.to_ref())
            .map_err(|e| anyhow!("svg open error: {}", e))?;

        let width = tree.svg_node().size.width().ceil() as u32;
        let height = tree.svg_node().size.height().ceil() as u32;
        let mut buf = tiny_skia::Pixmap::new(width, height).context("invalid pixmap size")?;
        resvg::render(&tree, usvg::FitTo::Original, buf.as_mut())
            .ok_or_else(|| anyhow!("cannot render svg"))?;

        Ok(Self {
            width,
            height,
            data: rgba_to_argb(buf.data()),
        })
    }

    pub fn as_image(&self) -> raqote::Image {
        raqote::Image {
            width: self.width as i32,
            height: self.height as i32,
            data: self.data.as_slice(),
        }
    }
}

fn rgba_to_argb(buf: &[u8]) -> Vec<u32> {
    debug_assert!(buf.len() % 4 == 0);

    let mut data = vec![];

    for chunk in buf.chunks(4) {
        let src =
            raqote::SolidSource::from_unpremultiplied_argb(chunk[3], chunk[0], chunk[1], chunk[2]);

        let a = u32::from(src.a) << 24;
        let r = u32::from(src.r) << 16;
        let g = u32::from(src.g) << 8;
        let b = u32::from(src.b);

        data.push(a | r | g | b);
    }

    data
}
