use super::*;

fn centre(body: &'static str) -> u8 {
    let icon = Box::leak(Box::new(Icon { name: "t", view: 16.0, body }));
    icon_coverage(icon, 32)[16 * 32 + 16]
}

#[test]
fn opacity_scales_an_elements_coverage() {
    assert_eq!(centre(r#"<circle cx="8" cy="8" r="6" fill="currentColor"/>"#), 255);
    let half = centre(r#"<circle cx="8" cy="8" r="6" fill="currentColor" opacity=".5"/>"#);
    assert!((126..=129).contains(&half), "opacity .5 is half coverage, got {half}");
    let fill = centre(r#"<circle cx="8" cy="8" r="6" fill="currentColor" fill-opacity="0.25"/>"#);
    assert!((62..=65).contains(&fill), "fill-opacity, got {fill}");
    let both = centre(r#"<circle cx="8" cy="8" r="6" fill="currentColor" opacity=".5" fill-opacity=".5"/>"#);
    assert!((62..=65).contains(&both), "they multiply, got {both}");
}

#[test]
fn stroke_opacity_leaves_the_fill_alone() {
    let body = r#"<rect x="0" y="0" width="16" height="16" fill="currentColor" stroke="currentColor" stroke-width="2" stroke-opacity=".3"/>"#;
    assert_eq!(centre(body), 255);
}

#[test]
fn owned_icon_rasterizes_and_keeps_distinct_identities() {
    let circle = OwnedIcon::new(
        "circle",
        16.0,
        r#"<circle cx="8" cy="8" r="6" fill="currentColor"/>"#,
    );
    let ring = OwnedIcon::new(
        "ring",
        16.0,
        r#"<circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="2"/>"#,
    );
    assert_ne!(circle, ring);
    assert_eq!(circle, circle.clone());
    assert_eq!(coverage_at(circle.view(), circle.body(), 32, 32.0)[16 * 32 + 16], 255);
    assert_eq!(coverage_at(ring.view(), ring.body(), 32, 32.0)[16 * 32 + 16], 0);
    let invalid = OwnedIcon::new("bad", 0.0, r#"<circle cx="8" cy="8" r="6" fill="currentColor"/>"#);
    assert!(coverage_at(invalid.view(), invalid.body(), 16, 16.0).iter().all(|&b| b == 0));

    let mut text = crate::text::TextSystem::default();
    let q_circle = text.icon_quad(&circle, 24, [0.0, 0.0]).expect("circle quad");
    let q_ring = text.icon_quad(&ring, 24, [0.0, 0.0]).expect("ring quad");
    assert_ne!(q_circle.uv, q_ring.uv, "distinct owned icons must occupy distinct atlas slots");
}
