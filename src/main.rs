extern crate rand;
extern crate sdl2;
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Rect, render::WindowCanvas, EventPump,
    IntegerOrSdlError::*, Sdl, VideoSubsystem,
};

use std::env;

mod chip8;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 512;

const KEYMAP: [Keycode; 16] = [
    Keycode::X,
    Keycode::Num1,
    Keycode::Num2,
    Keycode::Num3,
    Keycode::Q,
    Keycode::W,
    Keycode::E,
    Keycode::A,
    Keycode::S,
    Keycode::D,
    Keycode::Z,
    Keycode::C,
    Keycode::Num4,
    Keycode::R,
    Keycode::F,
    Keycode::V,
];

fn main() -> Result<(), String> {
    let context = sdl2::init()?;
    let video = context.video()?;
    let window = match {
        let mut window_builder = video.window("Alice's Chip-8 emulator", WIDTH, HEIGHT);

        window_builder.position_centered().build()
    } {
        Ok(window) => window,
        Err(error) => return Err(format!("Error building window: {}", error)),
    };

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("expected two arguments!");
    }

    let mut chip8 = chip8::Chip8::new();
    chip8.load(&args[1]);

    let mut canvas = match window.into_canvas().build() {
        Ok(canvas) => canvas,
        Err(IntegerOverflows(error, integer)) => {
            return Err(format!("{}: Caused by {}", error, integer))
        }
        Err(SdlError(error)) => return Err(error),
    };

    let mut pixels: [[u8; 2048]; 2048] = [[0; 2048]; 2048];

    let mut event_pump = context.event_pump()?;
    canvas.set_draw_color(Color::RGB(0, 0, 0));

    canvas.clear();
    canvas.present();

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(sdl2::pixels::PixelFormatEnum::ARGB8888, 64, 32)
        .unwrap();

    'running: loop {
        chip8.emulate_cycle();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

                Event::KeyDown {
                    keycode: Some(kc), ..
                } => {
                    for i in 0..16 {
                        if kc == KEYMAP[i] {
                            chip8.keypad[i] = 1;
                        }
                    }
                }

                Event::KeyUp {
                    keycode: Some(kc), ..
                } => {
                    for i in 0..16 {
                        if kc == KEYMAP[i] {
                            chip8.keypad[i] = 0;
                        }
                    }
                }
                _ => (),
            }
        }

        // If draw occurred, redraw SDL screen
        if chip8.draw_flag {
            chip8.draw_flag = false;
            canvas.clear();

            // Update SDL texture
            for (y, row) in chip8.gfx.iter().enumerate() {
                for (x, &col) in row.iter().enumerate() {
                    let x = (x as u32) * 30;
                    let y = (y as u32) * 30;

                    canvas.set_draw_color(color(col));
                    canvas.fill_rect(Rect::new(x as i32, y as i32, 20, 20))?;
                }
            }
            canvas.present();
        }
    }

    Ok(())
}

fn color(v: u8) -> Color {
    if v == 0 {
        Color::RGB(0, 0, 0)
    } else {
        Color::RGB(0, 250, 0)
    }
}
