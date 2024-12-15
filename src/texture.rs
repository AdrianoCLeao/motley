use glam::*;
use std::ffi::CString;

#[derive(Clone, Debug)]
pub struct Texture {
    data: Vec<u8>,
    width: u32,
    height: u32,
    channel_count: usize
}

pub fn load_texture(file_path: &str) -> Texture {
    let file_path = CString::new(file_path.as_bytes()).unwrap();

    unsafe {
        let mut width = 0;
        let mut height = 0;
        let mut channel_count = 0;
        let data = stb_image::stb_image::bindgen::stbi_load(
            file_path.as_ptr(),
            &mut width,
            &mut height,
            &mut channel_count,
            0,
        );

        assert!(!data.is_null(), "Failed to load texture.");
        let data: Vec<u8> = std::slice::from_raw_parts(
            data,
            (width * height * channel_count) as usize
        ).to_vec();

        Texture {
            data,
            width: width as u32,
            height: height as u32,
            channel_count: channel_count as usize
        }
    }
}