use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::VideoSubsystem;

pub struct Display {
    pixels: [[Pixel; 32]; 64],
    video_subsystem: VideoSubsystem,
    canvas: WindowCanvas,
}

impl Display {
    pub fn new(sdl_context: &sdl2::Sdl) -> Result<Display, String> {
        let video_subsystem = sdl_context.video()?;
        let window = match video_subsystem
            .window("CHIP-8 emulator", 640, 320)
            .position_centered()
            .build()
        {
            Ok(window) => window,
            Err(err) => return Err(err.to_string()),
        };
        let canvas = match window.into_canvas().build() {
            Ok(canvas) => canvas,
            Err(err) => return Err(err.to_string()),
        };

        let mut display = Display {
            pixels: [[Pixel::new(); 32]; 64],
            video_subsystem: video_subsystem,
            canvas: canvas,
        };

        // Initialize pixel positions.
        for i in 0..display.pixels.len() {
            for j in 0..display.pixels[i].len() {
                let pixel = &mut display.pixels[i][j];
                pixel.rect.set_x((i * 10) as i32);
                pixel.rect.set_y((j * 10) as i32);
            }
        }

        Ok(display)
    }

    pub fn set_pixels(self: &mut Self, framebuffer: &[u8; 256]) {
        self.canvas.set_draw_color(Color::RGB(0x22, 0x22, 0x22));
        self.canvas.clear();

        for col_byte in 0..8 {
            for row in 0..32 {
                let fb_byte = framebuffer[col_byte + row * 8];
                for pixel_x in 0..8 {
                    let pixel = &mut self.pixels[col_byte * 8 + pixel_x][row];
                    if fb_byte.wrapping_shr(7 - pixel_x as u32) & 1 == 1 {
                        self.canvas.set_draw_color(Color::RGB(0, 0xcc, 0x11));
                        self.canvas.fill_rect(pixel.rect).unwrap();
                    }
                }
            }
        }
    }

    pub fn present(self: &mut Self) {
        self.canvas.present();
    }
}

#[derive(Clone, Copy, Debug)]
struct Pixel {
    rect: Rect,
    colored: bool,
}

impl Pixel {
    fn new() -> Pixel {
        Pixel {
            rect: Rect::new(0, 0, 10, 10),
            colored: false,
        }
    }
}
