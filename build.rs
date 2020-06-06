use std::env;
use std::fs;
use std::io;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

use image::{GenericImage, GenericImageView, RgbaImage};
use rect_packer::{Packer, Rect};

fn parse(string: &str) -> Option<Vec<(&str, Rect)>> {
    let mut tokens = string.split_ascii_whitespace().peekable();
    let mut rects = Vec::new();
    while let Some(label) = tokens.next() {
        let x = tokens.next()?.parse::<i32>().ok()?;
        let y = tokens.next()?.parse::<i32>().ok()?;
        let width = tokens.next()?.parse::<i32>().ok()?;
        let height = tokens.next()?.parse::<i32>().ok()?;
        rects.push((
            label,
            Rect {
                x,
                y,
                width,
                height,
            },
        ));
    }
    Some(rects)
}

fn main() -> Result<(), io::Error> {
    let entries = fs::read_dir("res\\textures")?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    println!("cargo:rerun-if-changed=res\\textures");

    let mut sprites = Vec::new();

    entries.into_iter().for_each(|mut entry| {
        println!("cargo:rerun-if-changed={}", entry.to_str().unwrap());
        use std::ffi::OsStr;
        match entry.extension().and_then(OsStr::to_str) {
            Some("txt") => {
                let text = fs::read_to_string(&entry).unwrap();
                entry.set_extension("png");

                if let Some(i) = sprites.iter().position(|(path, _)| *path == entry) {
                    sprites.swap_remove(i);
                }

                let this_sprites = parse(&text).expect("some .txt is malformed");
                sprites.push((
                    entry,
                    this_sprites
                        .into_iter()
                        .map(|(a, b)| (a.to_owned(), b, Rect::new(0, 0, 0, 0)))
                        .collect::<Vec<_>>(),
                ));
            }
            Some("png") => {
                if sprites.iter().any(|(path, _)| *path == entry) {
                    return;
                }

                let (width, height) = image::image_dimensions(&entry).unwrap();
                let name = entry.file_stem().unwrap().to_str().unwrap().to_string();
                sprites.push((
                    entry,
                    vec![(
                        name,
                        Rect::new(0, 0, width as i32, height as i32),
                        Rect::new(0, 0, 0, 0),
                    )],
                ));
            }
            _ => (),
        }
    });

    let width = 2048;
    let height = 2048;

    let mut packer = Packer::new(rect_packer::Config {
        width,
        height,
        border_padding: 1,
        rectangle_padding: 2,
    });

    let mut atlas = RgbaImage::new(width as u32, height as u32);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let atlas_rs = PathBuf::from(&out_dir).join("atlas.rs");
    let mut atlas_rs = BufWriter::new(fs::File::create(atlas_rs).unwrap());
    // fs::write(
    //     &atlas_rs,
    //     r#"pub fn message() -> &'static str {
    //         "Hello, World!"
    //     }
    //     "#
    // ).unwrap();

    let mut rects = sprites
        .iter_mut()
        .map(|(_, x)| x.iter_mut().map(|(_, r, x)| (r.width, r.height, x)))
        .flatten()
        .collect::<Vec<(i32, i32, &mut Rect)>>();

    let mut i = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    rects.sort_by_cached_key(|(_, _, _)| {
        i = i.wrapping_mul(i >> 1);
        i % 777
    });

    for (width, height, output) in rects {
        eprintln!("  rect: {} {}", width, height);
        if let Some(packed) = packer.pack(width, height, false) {
            eprintln!(
                "packed: {} {} {} {}",
                packed.x, packed.y, packed.width, packed.height
            );
            *output = packed;
        } else {
            panic!("Packing failed!");
        }
    }

    for (path, rects) in sprites.iter() {
        let image = image::open(path).unwrap().to_rgba();

        for (name, rect, packed) in rects.iter() {
            eprintln!("copying {}...", name);
            let view = image.view(
                rect.x as u32,
                rect.y as u32,
                rect.width as u32,
                rect.height as u32,
            );
            atlas
                .copy_from(&view, packed.x as u32, packed.y as u32)
                .unwrap();

            let top = packed.top() as u32;
            let bottom = packed.bottom() as u32;
            let left = packed.left() as u32;
            let right = packed.right() as u32;

            atlas_rs.write_all(b"pub const ").unwrap();
            atlas_rs
                .write_all(name.replace(' ', "_").to_uppercase().as_bytes())
                .unwrap();
            atlas_rs
                .write_all(
                    format!(
                        ":[f32;4]=[{},{},{},{}];\n",
                        packed.x as f32 / width as f32,
                        packed.y as f32 / height as f32,
                        packed.width as f32 / width as f32,
                        packed.height as f32 / height as f32
                    )
                    .as_bytes(),
                )
                .unwrap();

            for x in left - 1..right + 1 {
                unsafe {
                    atlas.unsafe_put_pixel(x, top - 1, atlas.unsafe_get_pixel(x, top));
                    atlas.unsafe_put_pixel(x, bottom, atlas.unsafe_get_pixel(x, bottom - 1));
                }
            }

            for y in top - 1..bottom + 1 {
                unsafe {
                    atlas.unsafe_put_pixel(left - 1, y, atlas.unsafe_get_pixel(left, y));
                    atlas.unsafe_put_pixel(right, y, atlas.unsafe_get_pixel(right - 1, y));
                }
            }
        }
    }

    atlas
        .save(PathBuf::from(&out_dir).join("atlas.png"))
        .unwrap();

    // unimplemented!();

    Ok(())
}
