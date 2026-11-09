use std::io::Cursor;

use fieldguide_core::media::{PreparedMedia, prepare_image};
use fieldguide_core::{Error, Vault};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, GenericImageView, ImageFormat, RgbImage, RgbaImage};
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

/// The JPEG with an EXIF APP1 segment carrying only the orientation tag, as a camera writes it.
fn with_orientation(jpeg: &[u8], orientation: u8) -> Vec<u8> {
    let mut app1 = vec![0xFF, 0xE1, 0x00, 0x22];
    app1.extend_from_slice(b"Exif\0\0");
    // TIFF header, little endian, first IFD at offset 8; one entry: tag 0x0112, SHORT, count 1.
    app1.extend_from_slice(b"II\x2a\x00\x08\x00\x00\x00");
    app1.extend_from_slice(&[0x01, 0x00]);
    app1.extend_from_slice(&[0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00]);
    app1.extend_from_slice(&[orientation, 0x00, 0x00, 0x00]);
    app1.extend_from_slice(&[0, 0, 0, 0]);
    let mut out = jpeg[..2].to_vec();
    out.extend_from_slice(&app1);
    out.extend_from_slice(&jpeg[2..]);
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
fn photos_come_out_the_way_they_were_seen() {
    let wide = RgbImage::from_fn(4, 2, |x, _| image::Rgb([x as u8 * 60, 0, 0]));
    let mut jpeg = Cursor::new(Vec::new());
    wide.write_to(&mut jpeg, ImageFormat::Jpeg).unwrap();
    let plain = prepare_image(jpeg.get_ref(), Some("photo.jpg")).unwrap();
    assert_eq!(
        (plain.ext.as_str(), dims(&plain)),
        ("webp", (Some(4), Some(2)))
    );

    // Orientation 6 means "rotate 90 degrees clockwise to view": the stored picture is upright
    // and tall, since WebP has no place to carry the instruction.
    let sideways = prepare_image(&with_orientation(jpeg.get_ref(), 6), Some("photo.jpg")).unwrap();
    assert_eq!(dims(&sideways), (Some(2), Some(4)));
    let decoded = image::load_from_memory(&sideways.bytes).unwrap();
    assert_eq!(decoded.dimensions(), (2, 4));
}

#[test]
fn the_bytes_decide_the_format_and_the_name_only_helps_svg() {
    // A PNG called .svg is a PNG.
    let png = encoded(&checker(3), ImageFormat::Png);
    let stored = prepare_image(&png, Some("drawing.svg")).unwrap();
    assert_eq!(stored.ext, "webp");
    // Text that opens an svg tag is SVG whatever it is called.
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
    assert_eq!(prepare_image(svg, Some("pasted")).unwrap().ext, "svg");
    assert_eq!(prepare_image(svg, None).unwrap().ext, "svg");
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
