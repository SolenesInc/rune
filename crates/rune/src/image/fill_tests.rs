use super::*;

fn png(width: u32, height: u32) -> Vec<u8> {
    let source = image::RgbaImage::from_fn(width, height, |x, y| image::Rgba([(x % 251) as u8, (y % 241) as u8, ((x ^ y) % 239) as u8, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    source.write_to(&mut encoded, image::ImageFormat::Png).unwrap();
    encoded.into_inner()
}

fn held(image: &Image) -> usize {
    image.decoded().unwrap().unwrap().mips.iter().map(|m| m.pixels.len()).sum()
}

fn top(image: &Image) -> [u32; 2] {
    let mip = &image.decoded().unwrap().unwrap().mips[0];
    [mip.width, mip.height]
}

#[test]
fn a_large_image_asked_for_small_holds_only_the_mips_that_fill_its_size() {
    let bytes = png(4000, 2400);
    let cache = ImageCache::new(Arc::new(|| {}));
    let whole = cache.load(bytes.clone());
    let thumbnail = cache.load_mime(bytes, "image/png", 66.0, 2.0);
    assert_eq!(whole.wait(), Ok([4000, 2400]));
    assert_eq!(thumbnail.wait(), Ok([4000, 2400]), "the image keeps its own size");
    assert_eq!(top(&whole), [4000, 2400]);
    assert_eq!(top(&thumbnail), [250, 150], "the smallest level that fills 132 pixels square");
    assert!(held(&thumbnail) < 250 * 150 * 4 * 2, "{} bytes held", held(&thumbnail));
    assert!(held(&whole) > 4000 * 2400 * 4);

    let level = whole.decoded().unwrap().unwrap().mips.iter().position(|m| m.width == 250).unwrap();
    for (kept, full) in thumbnail.decoded().unwrap().unwrap().mips.iter().zip(&whole.decoded().unwrap().unwrap().mips[level..]) {
        assert_eq!(kept.pixels, full.pixels, "the same pixels the whole chain draws at that size");
    }
    let mut atlas = crate::image::pixels::ImageAtlas::default();
    let covering_132_square = [220.0, 132.0];
    let drawn = |atlas: &mut crate::image::pixels::ImageAtlas, image: &Image| atlas.place(image.pixels(), covering_132_square).unwrap().rect[2..].to_vec();
    assert_eq!(drawn(&mut atlas, &thumbnail), drawn(&mut atlas, &whole));
    assert_eq!(thumbnail.pixels().size(), (4000, 2400));
}

#[test]
fn a_file_asked_for_larger_decodes_again_and_a_small_image_keeps_every_level() {
    let dir = std::env::temp_dir().join(format!("rune-image-fill-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("shot.png");
    std::fs::write(&path, png(2000, 1200)).unwrap();
    let cache = ImageCache::new(Arc::new(|| {}));
    let small = cache.file(&path, 66.0, 2.0);
    let large = cache.file(&path, 400.0, 2.0);
    assert!(!Arc::ptr_eq(&small.0, &large.0), "a larger size is a picture of its own");
    assert_eq!((small.wait(), large.wait()), (Ok([2000, 1200]), Ok([2000, 1200])));
    assert_eq!(top(&small), [250, 150]);
    assert_eq!(top(&large), [2000, 1200], "no smaller level fills 800 pixels square");
    let icon = cache.load_mime(png(40, 20), "image/png", 66.0, 2.0);
    assert_eq!(icon.wait(), Ok([40, 20]));
    assert_eq!(icon.decoded().unwrap().unwrap().mips.len(), 6, "smaller than asked, it keeps every level");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_one_pixel_image_asked_for_one_pixel_keeps_its_pixel_and_draws() {
    let cache = ImageCache::new(Arc::new(|| {}));
    let dot = cache.load_mime(png(1, 1), "image/png", 0.5, 2.0);
    assert_eq!(dot.wait(), Ok([1, 1]));
    assert_eq!(dot.decoded().unwrap().unwrap().mips[0].pixels.len(), 4);
    let mut atlas = crate::image::pixels::ImageAtlas::default();
    assert!(atlas.place(dot.pixels(), [1.0, 1.0]).is_some());
}
