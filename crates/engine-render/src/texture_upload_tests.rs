use std::borrow::Cow;

use crate::texture_upload::{prepare_rgba8_upload_data, RGBA8_BYTES_PER_PIXEL};

#[test]
fn prepare_rgba8_upload_data_rejects_too_small_payload() {
    let width = 8;
    let height = 4;
    let expected_size = (width * RGBA8_BYTES_PER_PIXEL * height) as usize;
    let pixels = vec![0_u8; expected_size - 1];

    let error = prepare_rgba8_upload_data(width, height, &pixels)
        .expect_err("payload below expected size should fail");

    assert!(error.to_string().contains("texture payload too small"));
}

#[test]
fn prepare_rgba8_upload_data_keeps_borrowed_data_when_row_is_aligned() {
    let width = 64;
    let height = 2;
    let expected_size = (width * RGBA8_BYTES_PER_PIXEL * height) as usize;
    let pixels = vec![7_u8; expected_size + 64];

    let prepared =
        prepare_rgba8_upload_data(width, height, &pixels).expect("aligned upload should pass");

    assert_eq!(prepared.bytes_per_row, width * RGBA8_BYTES_PER_PIXEL);
    assert_eq!(prepared.rows_per_image, height);
    assert!(matches!(prepared.data, Cow::Borrowed(_)));
    assert_eq!(prepared.data.len(), expected_size);
}

#[test]
fn prepare_rgba8_upload_data_pads_rows_when_alignment_is_required() {
    let width = 3;
    let height = 2;
    let row_bytes = (width * RGBA8_BYTES_PER_PIXEL) as usize;
    let pixels: Vec<u8> = (0..row_bytes * height as usize)
        .map(|v| (v % 255) as u8)
        .collect();

    let prepared = prepare_rgba8_upload_data(width, height, &pixels)
        .expect("unaligned upload should be padded");

    assert_eq!(prepared.bytes_per_row, 256);
    assert_eq!(prepared.rows_per_image, height);
    assert!(matches!(prepared.data, Cow::Owned(_)));
    assert_eq!(prepared.data.len(), 256 * height as usize);

    assert_eq!(&prepared.data[0..row_bytes], &pixels[0..row_bytes]);
    assert_eq!(prepared.data[row_bytes..256], vec![0_u8; 256 - row_bytes]);

    let second_row_src_start = row_bytes;
    let second_row_src_end = row_bytes * 2;
    let second_row_dst_start = 256;
    let second_row_dst_end = 256 + row_bytes;
    assert_eq!(
        &prepared.data[second_row_dst_start..second_row_dst_end],
        &pixels[second_row_src_start..second_row_src_end]
    );
}
