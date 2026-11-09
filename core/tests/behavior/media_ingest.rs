use std::io::Cursor;

use fieldguide_core::media::{PreparedMedia, prepare_image};
use fieldguide_core::{Error, Vault};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageFormat, RgbaImage};
use tempfile::tempdir;

fn checker(size: u32) -> RgbaImage {
    RgbaImage::from_fn(size, size, |x, y| {
        if (x + y) % 2 == 0 {
            image::Rgba([200, 30, 30, 255])
        } else {
            image::Rgba([30, 30, 200, 255])
        }
    })
}

fn encoded(img: &RgbaImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, format).unwrap();
    out.into_inner()
}

fn animated_gif(frames: usize) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut out);
        encoder.set_repeat(Repeat::Infinite).unwrap();
        for i in 0..frames {
            let mut img = checker(8);
            img.put_pixel(0, 0, image::Rgba([i as u8, 0, 0, 255]));
            encoder
                .encode_frame(Frame::from_parts(
                    img,
                    0,
                    0,
                    Delay::from_numer_denom_ms(100, 1),
                ))
                .unwrap();
        }
    }
    out
}

fn dims(prepared: &PreparedMedia) -> (Option<i64>, Option<i64>) {
    (prepared.dimensions.width, prepared.dimensions.height)
}

#[test]
fn still_images_become_lossless_webp_with_their_size_and_the_same_pixels_deduplicate() {
    let pixels = checker(6);
    let png = prepare_image(&encoded(&pixels, ImageFormat::Png), Some("shot.png")).unwrap();
    assert_eq!(png.ext, "webp");
    assert_eq!(png.mime, "image/webp");
    assert_eq!(dims(&png), (Some(6), Some(6)));
    assert!(image::guess_format(&png.bytes).unwrap() == ImageFormat::WebP);

    // The same pixels from a BMP give the same WebP bytes, so one file serves both.
    let bmp = prepare_image(&encoded(&pixels, ImageFormat::Bmp), None).unwrap();
    assert_eq!(bmp.bytes, png.bytes);

    // Nothing is lost: decoding the stored file gives the source pixels back.
    let decoded = image::load_from_memory(&png.bytes).unwrap().to_rgba8();
    assert_eq!(decoded.as_raw(), pixels.as_raw());

    let still_gif = prepare_image(&encoded(&pixels, ImageFormat::Gif), Some("a.gif")).unwrap();
    assert_eq!(still_gif.ext, "webp");
}

#[test]
fn animated_gifs_webp_and_svg_are_kept_as_they_are() {
    let gif = prepare_image(&animated_gif(3), Some("walk.gif")).unwrap();
    assert_eq!((gif.ext.as_str(), gif.mime.as_str()), ("gif", "image/gif"));
    assert_eq!(dims(&gif), (Some(8), Some(8)));

    let source_webp = prepare_image(&encoded(&checker(5), ImageFormat::Png), None).unwrap();
    let again = prepare_image(&source_webp.bytes, Some("x.webp")).unwrap();
    assert_eq!(again.bytes, source_webp.bytes);
    assert_eq!(dims(&again), (Some(5), Some(5)));

    let svg = b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
    let kept = prepare_image(svg, Some("icon.svg")).unwrap();
    assert_eq!(
        (kept.ext.as_str(), kept.mime.as_str()),
        ("svg", "image/svg+xml")
    );
    assert_eq!(kept.bytes, svg);
}

#[test]
fn bytes_that_are_not_an_image_are_refused() {
    let err = prepare_image(b"just some text", Some("notes.txt")).unwrap_err();
    assert!(matches!(err, Error::Invalid(_)), "{err:?}");
    assert!(err.to_string().contains("PNG"), "{err}");
}

#[test]
fn ingested_images_are_stored_once_and_the_primary_image_leads_the_gallery() {
    let dir = tempdir().unwrap();
    let mut v = Vault::open(&dir.path().join("vault").join("vault.db")).unwrap();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone", "");

    let pixels = checker(4);
    let first = v
        .ingest_image(&encoded(&pixels, ImageFormat::Png), Some("a.png"))
        .unwrap();
    let second = v
        .ingest_image(&encoded(&pixels, ImageFormat::Bmp), Some("b.bmp"))
        .unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(first.ext, "webp");
    assert!(v.media_file_path(&first).unwrap().exists());

    let other = v
        .ingest_image(&encoded(&checker(3), ImageFormat::Png), None)
        .unwrap();
    v.attach_media(&stone, &first.id, None).unwrap();
    v.attach_media(&stone, &other.id, None).unwrap();
    v.set_primary_media(&stone, &other.id).unwrap();
    let gallery = v.entry_gallery(&stone).unwrap();
    assert_eq!(gallery[0].id, other.id);
    assert_eq!(gallery[1].id, first.id);

    let path = dir.path().join("vault").join("drop.png");
    std::fs::write(&path, encoded(&checker(7), ImageFormat::Png)).unwrap();
    let dropped = v.ingest_image_file(&path).unwrap();
    assert_eq!((dropped.width, dropped.height), (Some(7), Some(7)));
}

use crate::common::*;
