use std::env;
use std::fs::File;
use std::io::Read;

extern crate sdl2;

use std::time::Duration;

pub mod display;
pub mod input;
pub mod interpreter;
pub mod sound;

fn load_bytes_from_file(path: &String) -> Result<Vec<u8>, String> {
    let mut bytes: Vec<u8> = Vec::new();
    let file = match File::open(path.as_str()) {
        Ok(file) => file,
        _ => return Err("Failed to open file.".to_string()),
    };
    file.take(0xffff).read_to_end(&mut bytes).unwrap();

    Ok(bytes)
}

fn main() {
    let mut step_mode = false;

    let args: Vec<String> = env::args().collect();

    let mut rom_path: String = "".to_string();
    for arg in args.into_iter() {
        if arg == "--step" {
            step_mode = true;
        } else {
            rom_path = arg;
        }
    }

    let sdl_context = sdl2::init().unwrap();
    let mut display = display::Display::new(&sdl_context).unwrap();
    let mut sound = sound::Sound::new(&sdl_context).unwrap();
    let mut input = input::Input::new();

    let mut interpreter =
        interpreter::Interpreter::new(load_bytes_from_file(&rom_path).unwrap().as_slice());

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut step_mode_active = step_mode;
    let mut next_instruction;
    'running: loop {
        next_instruction = !step_mode_active;

        input.collect(&mut event_pump);

        if input.quit {
            break 'running;
        }

        if input.step_mode_changed {
            step_mode_active = !step_mode_active;
            if step_mode_active {
                next_instruction = false;
            }
        }

        if input.step_to_next_instruction {
            // If step mode is not active, always move to the next instruction. Otherwise,
            // only do it when the key is pressed.
            next_instruction = !step_mode_active || !next_instruction;
        }

        if input.print_state {
            interpreter.print_state();
        }

        if next_instruction {
            interpreter.execute_next_instruction(&mut display, &mut sound, &input).unwrap();
        }

        display.present();
        ::std::thread::sleep(Duration::from_micros(16666));
    }
}
