use minifb::Window;

use crate::gfx::{HEIGHT, WIDTH};

pub struct Presenter {
    #[cfg(windows)]
    hardware: Option<OpenGlPresenter>,
}

impl Presenter {
    pub fn new(window: &Window, hardware: bool) -> Result<Self, String> {
        let mut presenter = Self {
            #[cfg(windows)]
            hardware: None,
        };
        presenter.set_hardware(window, hardware)?;
        Ok(presenter)
    }

    pub fn set_hardware(&mut self, window: &Window, enabled: bool) -> Result<(), String> {
        #[cfg(windows)]
        {
            if enabled == self.hardware.is_some() {
                return Ok(());
            }
            self.hardware = if enabled {
                Some(OpenGlPresenter::new(window)?)
            } else {
                None
            };
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = window;
            if enabled {
                Err(
                    "OpenGL hardware acceleration is currently available on Windows only"
                        .to_owned(),
                )
            } else {
                Ok(())
            }
        }
    }

    pub fn present(&mut self, window: &mut Window, pixels: &[u32]) -> Result<(), String> {
        #[cfg(windows)]
        if let Some(hardware) = &mut self.hardware {
            window.update();
            return hardware.present(window.get_size(), pixels);
        }
        window
            .update_with_buffer(pixels, WIDTH, HEIGHT)
            .map_err(|error| error.to_string())
    }
}

pub fn letterbox_viewport(
    window_width: usize,
    window_height: usize,
) -> (usize, usize, usize, usize) {
    if window_width == 0 || window_height == 0 {
        return (0, 0, 0, 0);
    }
    let (width, height) =
        if window_width.saturating_mul(HEIGHT) <= window_height.saturating_mul(WIDTH) {
            (window_width, window_width.saturating_mul(HEIGHT) / WIDTH)
        } else {
            (window_height.saturating_mul(WIDTH) / HEIGHT, window_height)
        };
    (
        (window_width - width) / 2,
        (window_height - height) / 2,
        width,
        height,
    )
}

#[cfg(windows)]
mod windows_gl {
    use std::{ffi::c_void, mem::size_of, ptr};

    use minifb::Window;

    use super::letterbox_viewport;
    use crate::gfx::{HEIGHT, WIDTH};

    type Handle = *mut c_void;

    const PFD_DRAW_TO_WINDOW: u32 = 0x0000_0004;
    const PFD_SUPPORT_OPENGL: u32 = 0x0000_0020;
    const PFD_DOUBLEBUFFER: u32 = 0x0000_0001;
    const PFD_TYPE_RGBA: u8 = 0;
    const PFD_MAIN_PLANE: u8 = 0;
    const GL_TEXTURE_2D: u32 = 0x0DE1;
    const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
    const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
    const GL_NEAREST: i32 = 0x2600;
    const GL_RGBA: u32 = 0x1908;
    const GL_UNSIGNED_BYTE: u32 = 0x1401;
    const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
    const GL_QUADS: u32 = 0x0007;
    const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;

    #[repr(C)]
    struct PixelFormatDescriptor {
        size: u16,
        version: u16,
        flags: u32,
        pixel_type: u8,
        color_bits: u8,
        red_bits: u8,
        red_shift: u8,
        green_bits: u8,
        green_shift: u8,
        blue_bits: u8,
        blue_shift: u8,
        alpha_bits: u8,
        alpha_shift: u8,
        accumulation_bits: u8,
        accumulation_red_bits: u8,
        accumulation_green_bits: u8,
        accumulation_blue_bits: u8,
        accumulation_alpha_bits: u8,
        depth_bits: u8,
        stencil_bits: u8,
        auxiliary_buffers: u8,
        layer_type: u8,
        reserved: u8,
        layer_mask: u32,
        visible_mask: u32,
        damage_mask: u32,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetDC(window: Handle) -> Handle;
        fn ReleaseDC(window: Handle, device_context: Handle) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn ChoosePixelFormat(
            device_context: Handle,
            descriptor: *const PixelFormatDescriptor,
        ) -> i32;
        fn SetPixelFormat(
            device_context: Handle,
            format: i32,
            descriptor: *const PixelFormatDescriptor,
        ) -> i32;
        fn GetPixelFormat(device_context: Handle) -> i32;
        fn SwapBuffers(device_context: Handle) -> i32;
    }

    #[link(name = "opengl32")]
    unsafe extern "system" {
        fn wglCreateContext(device_context: Handle) -> Handle;
        fn wglDeleteContext(context: Handle) -> i32;
        fn wglMakeCurrent(device_context: Handle, context: Handle) -> i32;
        fn glBegin(mode: u32);
        fn glBindTexture(target: u32, texture: u32);
        fn glClear(mask: u32);
        fn glClearColor(red: f32, green: f32, blue: f32, alpha: f32);
        fn glDeleteTextures(count: i32, textures: *const u32);
        fn glEnable(capability: u32);
        fn glEnd();
        fn glGenTextures(count: i32, textures: *mut u32);
        fn glPixelStorei(name: u32, value: i32);
        fn glTexCoord2f(s: f32, t: f32);
        fn glTexImage2D(
            target: u32,
            level: i32,
            internal_format: i32,
            width: i32,
            height: i32,
            border: i32,
            format: u32,
            pixel_type: u32,
            pixels: *const c_void,
        );
        fn glTexParameteri(target: u32, name: u32, value: i32);
        fn glTexSubImage2D(
            target: u32,
            level: i32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            format: u32,
            pixel_type: u32,
            pixels: *const c_void,
        );
        fn glVertex2f(x: f32, y: f32);
        fn glViewport(x: i32, y: i32, width: i32, height: i32);
    }

    pub struct OpenGlPresenter {
        window: Handle,
        device_context: Handle,
        context: Handle,
        texture: u32,
        rgba: Vec<u8>,
    }

    impl OpenGlPresenter {
        pub fn new(window: &Window) -> Result<Self, String> {
            let window_handle = window.get_window_handle();
            if window_handle.is_null() {
                return Err("window did not expose a native Windows handle".to_owned());
            }
            let device_context = unsafe { GetDC(window_handle) };
            if device_context.is_null() {
                return Err("GetDC failed while enabling OpenGL".to_owned());
            }
            let descriptor = PixelFormatDescriptor {
                size: size_of::<PixelFormatDescriptor>() as u16,
                version: 1,
                flags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
                pixel_type: PFD_TYPE_RGBA,
                color_bits: 32,
                red_bits: 0,
                red_shift: 0,
                green_bits: 0,
                green_shift: 0,
                blue_bits: 0,
                blue_shift: 0,
                alpha_bits: 8,
                alpha_shift: 0,
                accumulation_bits: 0,
                accumulation_red_bits: 0,
                accumulation_green_bits: 0,
                accumulation_blue_bits: 0,
                accumulation_alpha_bits: 0,
                depth_bits: 0,
                stencil_bits: 0,
                auxiliary_buffers: 0,
                layer_type: PFD_MAIN_PLANE,
                reserved: 0,
                layer_mask: 0,
                visible_mask: 0,
                damage_mask: 0,
            };
            let existing_format = unsafe { GetPixelFormat(device_context) };
            if existing_format == 0 {
                let format = unsafe { ChoosePixelFormat(device_context, &descriptor) };
                if format == 0
                    || unsafe { SetPixelFormat(device_context, format, &descriptor) } == 0
                {
                    unsafe { ReleaseDC(window_handle, device_context) };
                    return Err("Windows could not select an OpenGL pixel format".to_owned());
                }
            }
            let context = unsafe { wglCreateContext(device_context) };
            if context.is_null() || unsafe { wglMakeCurrent(device_context, context) } == 0 {
                if !context.is_null() {
                    unsafe { wglDeleteContext(context) };
                }
                unsafe { ReleaseDC(window_handle, device_context) };
                return Err("Windows could not create an OpenGL rendering context".to_owned());
            }

            let mut texture = 0;
            unsafe {
                glClearColor(0.0, 0.0, 0.0, 1.0);
                glEnable(GL_TEXTURE_2D);
                glGenTextures(1, &mut texture);
                glBindTexture(GL_TEXTURE_2D, texture);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
                glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
                glTexImage2D(
                    GL_TEXTURE_2D,
                    0,
                    GL_RGBA as i32,
                    WIDTH as i32,
                    HEIGHT as i32,
                    0,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    ptr::null(),
                );
            }
            Ok(Self {
                window: window_handle,
                device_context,
                context,
                texture,
                rgba: vec![0; WIDTH * HEIGHT * 4],
            })
        }

        pub fn present(
            &mut self,
            window_size: (usize, usize),
            pixels: &[u32],
        ) -> Result<(), String> {
            if pixels.len() != WIDTH * HEIGHT {
                return Err("OpenGL presenter received an invalid framebuffer size".to_owned());
            }
            for (source, target) in pixels.iter().zip(self.rgba.chunks_exact_mut(4)) {
                target[0] = (source >> 16) as u8;
                target[1] = (source >> 8) as u8;
                target[2] = *source as u8;
                target[3] = 255;
            }
            if unsafe { wglMakeCurrent(self.device_context, self.context) } == 0 {
                return Err("OpenGL context could not be made current".to_owned());
            }
            let (x, y, width, height) = letterbox_viewport(window_size.0, window_size.1);
            unsafe {
                glClear(GL_COLOR_BUFFER_BIT);
                glViewport(x as i32, y as i32, width as i32, height as i32);
                glBindTexture(GL_TEXTURE_2D, self.texture);
                glTexSubImage2D(
                    GL_TEXTURE_2D,
                    0,
                    0,
                    0,
                    WIDTH as i32,
                    HEIGHT as i32,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    self.rgba.as_ptr().cast(),
                );
                glBegin(GL_QUADS);
                glTexCoord2f(0.0, 1.0);
                glVertex2f(-1.0, -1.0);
                glTexCoord2f(1.0, 1.0);
                glVertex2f(1.0, -1.0);
                glTexCoord2f(1.0, 0.0);
                glVertex2f(1.0, 1.0);
                glTexCoord2f(0.0, 0.0);
                glVertex2f(-1.0, 1.0);
                glEnd();
            }
            if unsafe { SwapBuffers(self.device_context) } == 0 {
                return Err("SwapBuffers failed while presenting the OpenGL frame".to_owned());
            }
            Ok(())
        }
    }

    impl Drop for OpenGlPresenter {
        fn drop(&mut self) {
            unsafe {
                wglMakeCurrent(self.device_context, self.context);
                glDeleteTextures(1, &self.texture);
                wglMakeCurrent(ptr::null_mut(), ptr::null_mut());
                wglDeleteContext(self.context);
                ReleaseDC(self.window, self.device_context);
            }
        }
    }
}

#[cfg(windows)]
use windows_gl::OpenGlPresenter;

#[cfg(test)]
mod tests {
    use super::letterbox_viewport;

    #[test]
    fn viewport_preserves_the_java_three_by_two_canvas_ratio() {
        assert_eq!(letterbox_viewport(1152, 768), (0, 0, 1152, 768));
        assert_eq!(letterbox_viewport(1200, 768), (24, 0, 1152, 768));
        assert_eq!(letterbox_viewport(800, 800), (0, 133, 800, 533));
    }
}
