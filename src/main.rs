use std::env;
use std::fs::File;
use std::io::{Read};
use rand::{Rng, rngs::ThreadRng};

extern crate sdl2;

use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioStatus};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::Duration;

#[derive(Debug)]
struct State {
    framebuffer: [u8; 256],
    memory: [u8; 0xfff],
    registers: [u8; 16],
    stack: [u16; 0xf],
    key_states: [u8; 0x10],
    memory_register: u16,
    delay_timer: u8,
    sound_timer: u8,
    // internal registers (not available for CHIP-8 programs)
    program_counter: u16,
    stack_pointer: usize,
    random_number_generator: ThreadRng,
}

impl State {
    fn new() -> State {
        let mut state = State {
            framebuffer: [0; 256],
            memory: [0; 0xfff],
            registers: [0; 16],
            stack: [0; 0xf],
            memory_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            program_counter: 0,
            stack_pointer: 0,
            key_states: [0; 0x10],
            random_number_generator: rand::thread_rng()
        };

        // Initialize hard-coded digit sprites. These should reside in the interpreter
        // area of memory - 0x000 to 0x1ff. We put them right at the beginning, at addresses 0x0000 - 0x0050.
        state.memory[0..5].copy_from_slice(&[0xf0, 0x90, 0x90, 0x90, 0xf0]); // 0
        state.memory[5..10].copy_from_slice(&[0x20, 0x60, 0x20, 0x20, 0x70]); // 1
        state.memory[10..15].copy_from_slice(&[0xf0, 0x10, 0xf0, 0x80, 0xf0]); // 2
        state.memory[15..20].copy_from_slice(&[0xf0, 0x10, 0xf0, 0x10, 0xf0]); // 3
        state.memory[20..25].copy_from_slice(&[0x90, 0x90, 0xf0, 0x10, 0x10]); // 4
        state.memory[25..30].copy_from_slice(&[0xf0, 0x80, 0xf0, 0x10, 0xf0]); // 5
        state.memory[30..35].copy_from_slice(&[0xf0, 0x80, 0xf0, 0x90, 0xf0]); // 6
        state.memory[35..40].copy_from_slice(&[0xf0, 0x10, 0x20, 0x40, 0x40]); // 7
        state.memory[40..45].copy_from_slice(&[0xf0, 0x90, 0xf0, 0x90, 0xf0]); // 8
        state.memory[45..50].copy_from_slice(&[0xf0, 0x90, 0xf0, 0x10, 0xf0]); // 9
        state.memory[50..55].copy_from_slice(&[0xf0, 0x90, 0xf0, 0x90, 0x90]); // A
        state.memory[55..60].copy_from_slice(&[0xe0, 0x90, 0xe0, 0x90, 0xe0]); // B
        state.memory[60..65].copy_from_slice(&[0xf0, 0x80, 0x80, 0x80, 0xf0]); // C
        state.memory[65..70].copy_from_slice(&[0xe0, 0x90, 0x90, 0x90, 0xe0]); // D
        state.memory[70..75].copy_from_slice(&[0xf0, 0x80, 0xf0, 0x80, 0xf0]); // E
        state.memory[75..80].copy_from_slice(&[0xf0, 0x80, 0xf0, 0x80, 0x80]); // F

        state
    }
}

struct KeyState {
    up: u8,
    pressed: u8,
    down: u8,
}

const KEY_PRESSED: u8 = 0b10;
const KEY_DOWN: u8 = 0b11;
const KEY_UP: u8 = 0b01;

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

type Address = u16;
type Register = usize;
type Value = u8;
type Nibble = u8;

#[derive(Debug)]
enum Instruction {
    // For descriptions, see http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
    INVALID,
    SYS,
    CLS,
    RET,
    JP(Address),
    CALL(Address),
    SERV(Register, Value),
    SNERV(Register, Value),
    SERR(Register, Register),
    LDRV(Register, Value),
    ADDRV(Register, Value),
    LDRR(Register, Register),
    ORRR(Register, Register),
    ANDRR(Register, Register),
    XORRR(Register, Register),
    ADDRR(Register, Register),
    SUBRR(Register, Register),
    SHR(Register, Register),
    SUBN(Register, Register),
    SHL(Register, Register),
    SNERR(Register, Register),
    LDI(Address),
    JP0A(Address),
    RND(Register, Value),
    DRW(Register, Register, Nibble),
    SKP(Register),
    SKNP(Register),
    LDRDT(Register),
    LDRK(Register),
    LDDTR(Register),
    LDSTR(Register),
    ADDI(Register),
    LDF(Register),
    LDB(Register),
    LDIR(Register),
    LDRI(Register),
}

fn decode_opcode(opcode: u16) -> Instruction {
    match (opcode & 0xf000) >> 12 {
        0 => match opcode & 0xfff {
            0x0E0 => Instruction::CLS,
            0x0EE => Instruction::RET,
            _ => Instruction::SYS,
        },
        1 => Instruction::JP(opcode & 0xfff),
        2 => Instruction::CALL(opcode & 0xfff),
        3 => Instruction::SERV(
            ((opcode & 0xf00) >> 8) as Register,
            (opcode & 0xff) as Value,
        ),
        4 => Instruction::SNERV(
            ((opcode & 0xf00) >> 8) as Register,
            (opcode & 0xff) as Value,
        ),
        5 => Instruction::SERR(
            ((opcode & 0xf00) >> 8) as Register,
            ((opcode & 0xf0) >> 4) as Register,
        ),
        6 => Instruction::LDRV(
            ((opcode & 0xf00) >> 8) as Register,
            (opcode & 0xff) as Value,
        ),
        7 => Instruction::ADDRV(
            ((opcode & 0xf00) >> 8) as Register,
            (opcode & 0xff) as Value,
        ),
        8 => match opcode & 0xf {
            0 => Instruction::LDRR(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            1 => Instruction::ORRR(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            2 => Instruction::ANDRR(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            3 => Instruction::XORRR(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            4 => Instruction::ADDRR(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            5 => Instruction::SUBRR(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            6 => Instruction::SHR(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            7 => Instruction::SUBN(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            0xe => Instruction::SHL(
                ((opcode & 0xf00) >> 8) as Register,
                ((opcode & 0xf0) >> 4) as Register,
            ),
            _ => Instruction::INVALID,
        },
        9 => Instruction::SNERR(
            ((opcode & 0xf00) >> 8) as Register,
            ((opcode & 0xf0) >> 4) as Register,
        ),
        0xa => Instruction::LDI(opcode & 0xfff),
        0xb => Instruction::JP0A(opcode & 0xfff),
        0xc => Instruction::RND(
            ((opcode & 0xf00) >> 8) as Register,
            (opcode & 0xff) as Value,
        ),
        0xd => Instruction::DRW(
            ((opcode & 0xf00) >> 8) as Register,
            ((opcode & 0xf0) >> 4) as Register,
            (opcode & 0xf) as Nibble,
        ),
        0xe => match opcode & 0xff {
            0x9E => Instruction::SKP(((opcode & 0xf00) >> 8) as Register),
            0xA1 => Instruction::SKNP(((opcode & 0xf00) >> 8) as Register),
            _ => Instruction::INVALID,
        },
        0xf => match opcode & 0xff {
            0x07 => Instruction::LDRDT(((opcode & 0xf00) >> 8) as Register),
            0x0A => Instruction::LDRK(((opcode & 0xf00) >> 8) as Register),
            0x15 => Instruction::LDDTR(((opcode & 0xf00) >> 8) as Register),
            0x18 => Instruction::LDSTR(((opcode & 0xf00) >> 8) as Register),
            0x1E => Instruction::ADDI(((opcode & 0xf00) >> 8) as Register),
            0x29 => Instruction::LDF(((opcode & 0xf00) >> 8) as Register),
            0x33 => Instruction::LDB(((opcode & 0xf00) >> 8) as Register),
            0x55 => Instruction::LDIR(((opcode & 0xf00) >> 8) as Register),
            0x65 => Instruction::LDRI(((opcode & 0xf00) >> 8) as Register),
            _ => panic!("Invalid opcode: {:?}", opcode),
        },
        _ => Instruction::INVALID,
    }
}

fn execute_instruction(instruction: Instruction, state: &mut State) {
    state.program_counter += 2;
    match instruction {
        Instruction::INVALID => (),
        Instruction::SYS => (),
        Instruction::CLS => {
            state.framebuffer = [0; 256];
        }
        Instruction::RET => {
            state.program_counter = state.stack[state.stack_pointer];
            state.stack_pointer -= 1;
        }
        Instruction::JP(address) => {
            state.program_counter = address;
        }
        Instruction::CALL(address) => {
            state.stack_pointer += 1;
            state.stack[state.stack_pointer] = state.program_counter;
            state.program_counter = address;
        }
        Instruction::SERV(register, value) => {
            if state.registers[register] == value {
                state.program_counter += 2;
            }
        }
        Instruction::SNERV(register, value) => {
            if state.registers[register] != value {
                state.program_counter += 2;
            }
        }
        Instruction::SERR(register0, register1) => {
            if state.registers[register0] == state.registers[register1] {
                state.program_counter += 2;
            }
        }
        Instruction::LDRV(register, value) => {
            state.registers[register] = value;
        }
        Instruction::ADDRV(register, value) => {
            let sum: u16 = state.registers[register] as u16 + value as u16;
            state.registers[0xf] = if sum > 255 { 1 } else { 0 };
            state.registers[register] = state.registers[register].wrapping_add(value);
        }
        Instruction::LDRR(register0, register1) => {
            state.registers[register0] = state.registers[register1];
        }
        Instruction::ORRR(register0, register1) => {
            state.registers[register0] |= state.registers[register1];
        }
        Instruction::ANDRR(register0, register1) => {
            state.registers[register0] &= state.registers[register1];
        }
        Instruction::XORRR(register0, register1) => {
            state.registers[register0] ^= state.registers[register1];
        }
        Instruction::ADDRR(register0, register1) => {
            let sum: u16 = state.registers[register1] as u16 + state.registers[register0] as u16;
            state.registers[0xf] = if sum > 255 { 1 } else { 0 };
            state.registers[register0] =
                state.registers[register0].wrapping_add(state.registers[register1]);
        }
        Instruction::SUBRR(register0, register1) => {
            let diff: i16 = state.registers[register0] as i16 - state.registers[register1] as i16;
            state.registers[0xf] = if diff > 0 { 1 } else { 0 };
            state.registers[register0] =
                state.registers[register0].wrapping_sub(state.registers[register1]);
        }
        Instruction::SHR(register0, _) => {
            state.registers[0xf] = if state.registers[register0] & 1 == 1 {
                1
            } else {
                0
            };
            state.registers[register0] >>= 1;
        }
        Instruction::SUBN(register0, register1) => {
            let diff: i16 = state.registers[register1] as i16 - state.registers[register0] as i16;
            state.registers[0xf] = if diff > 0 { 1 } else { 0 };
            state.registers[register0] =
                state.registers[register1].wrapping_sub(state.registers[register0]);
        }
        Instruction::SHL(register0, _register1) => {
            state.registers[0xf] = if state.registers[register0] & 0x80 == 0x80 {
                1
            } else {
                0
            };
            state.registers[register0] <<= 1;
        }
        Instruction::SNERR(register0, register1) => {
            if state.registers[register0] != state.registers[register1] {
                state.program_counter += 2;
            }
        }
        Instruction::LDI(address) => {
            state.memory_register = address;
        }
        Instruction::JP0A(address) => {
            state.program_counter = address + state.registers[0] as u16;
        }
        Instruction::RND(register, value) => {
            let random_number = state.random_number_generator.gen_range(0..=255);
            state.registers[register] = random_number & value;
        }
        Instruction::DRW(register0, register1, nibble) => {
            let screen_x = state.registers[register0] as usize;
            for row in 0..nibble {
                let screen_y = (state.registers[register1] + row) as usize;
                let bit_offset = screen_x % 8;

                let sprite_byte = state.memory[(state.memory_register + row as u16) as usize];
                let sprite_bits: u16 = (sprite_byte as u16) << (8 - bit_offset);

                let fb_byte_idx = (screen_x / 8 + screen_y * 8) % 256;
                state.framebuffer[fb_byte_idx] ^= (sprite_bits >> 8) as u8;
                if fb_byte_idx == state.framebuffer.len() - 1 {
                    state.framebuffer[0] ^= sprite_bits as u8;
                } else {
                    state.framebuffer[fb_byte_idx + 1] ^= sprite_bits as u8;
                }
            }
        }
        Instruction::SKP(register) => {
            if state.key_states[state.registers[register] as usize] == 1 {
                state.program_counter += 2;
            }
        }
        Instruction::SKNP(register) => {
            if state.key_states[state.registers[register] as usize] != 1 {
                state.program_counter += 2;
            }
        }
        Instruction::LDRDT(register) => {
            state.registers[register] = state.delay_timer;
        }
        Instruction::LDRK(register) => {
            match state.key_states.iter().position(|&k| k == KEY_PRESSED) {
                Some(key) => state.registers[register] = key as u8,
                _ => state.program_counter -= 2,
            }
        }
        Instruction::LDDTR(register) => {
            state.delay_timer = state.registers[register];
        }
        Instruction::LDSTR(register) => {
            state.sound_timer = state.registers[register];
        }
        Instruction::ADDI(register) => {
            state.memory_register = state
                .memory_register
                .wrapping_add(state.registers[register] as u16);
        }
        Instruction::LDF(register) => {
            state.memory_register = (state.registers[register] * 5) as u16;
        }
        Instruction::LDB(register) => {
            // TODO
            let value: f32 = state.registers[register] as f32;
            let hundreds = (value / 100.0).floor();
            let tens = ((value - hundreds * 100.0) / 10.0).floor();
            let ones = (value - hundreds * 100.0 - tens * 10.0).floor();
            println!("BCD of {}: {}, {}, {}", value, hundreds, tens, ones);
            state.memory[state.memory_register as usize] = hundreds as u8;
            state.memory[state.memory_register as usize + 1] = tens as u8;
            state.memory[state.memory_register as usize + 2] = ones as u8;
        }
        Instruction::LDIR(register) => {
            let num_registers = register + 1 as usize;
            let mem_start = state.memory_register as usize;
            let mem_end = (mem_start + num_registers) as usize;
            state.memory[mem_start..mem_end].copy_from_slice(&state.registers[0..num_registers]);
        }
        Instruction::LDRI(register) => {
            let num_registers = register + 1 as usize;
            let mem_start = state.memory_register as usize;
            let mem_end = (mem_start + num_registers) as usize;
            state.registers[0..num_registers as usize]
                .copy_from_slice(&state.memory[mem_start..mem_end]);
        }
    }
}

fn print_state(state: &State) {
    print!(
        "=================
Registers: {:?}
Memory register: {:?}
Stack: {:?}
Keys: {:?}
Delay timer: {}
=================
",
        state.registers, state.memory_register, state.stack, state.key_states, state.delay_timer
    );

    println!("Memory at memory register:");
    for i in 0..32 {
        print!("{:3X}: ", state.memory_register + i * 16);
        for j in 0..16 {
            print!(
                "{:2X} ",
                state.memory[(state.memory_register + j + i * 16) as usize]
            );
        }
        print!("\n");
    }

    // Print the framebuffer.
    // for i in 0..32 {
    //     for j in 0..8 {
    //         print!("{:08b}", state.framebuffer[j + i * 8]);
    //     }
    //     print!("\n");
    // }
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() {
    let mut state = State::new();

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
    let video_subsystem = sdl_context.video().unwrap();
    
    // Setup audio.
    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None
    };
    let audio_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.25
        }
    }).unwrap();

    let window = video_subsystem
        .window("CHIP-8 emulator", 640, 320)
        .position_centered()
        .build()
        .unwrap();

    {
        // load ROM.
        let mut opcode_buffer: Vec<u8> = Vec::new();
        let file = match File::open(rom_path.as_str()) {
            Ok(file) => file,
            _ => panic!("Failed to open ROM: {}", rom_path),
        };
        file.take(0xffff).read_to_end(&mut opcode_buffer).unwrap();

        let magic_string = match std::str::from_utf8(&opcode_buffer[0..3]) {
            Ok(magic_string) => magic_string,
            Err(..) => "",
        };
        if magic_string == "C8P" {
            state.memory[0x200..0x200 + (opcode_buffer.len() - 3)]
                .copy_from_slice(&opcode_buffer[3..]);
        } else {
            state.memory[0x200..0x200 + (opcode_buffer.len())].copy_from_slice(&opcode_buffer);
        }

        state.program_counter = 0x200;
    }

    let mut pixels = [[Pixel::new(); 32]; 64];

    // Initialize pixel positions.
    for i in 0..pixels.len() {
        for j in 0..pixels[i].len() {
            let pixel = &mut pixels[i][j];
            pixel.rect.set_x((i * 10) as i32);
            pixel.rect.set_y((j * 10) as i32);
        }
    }

    let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut step_mode_active = step_mode;
    let mut next_instruction;
    'running: loop {
        next_instruction = !step_mode_active;

        canvas.set_draw_color(Color::RGB(0x22, 0x22, 0x22));
        canvas.clear();

        // Translate state.framebuffer to pixels.
        for col_byte in 0..8 {
            for row in 0..32 {
                let fb_byte = state.framebuffer[col_byte + row * 8];
                for pixel_x in 0..8 {
                    let pixel = &mut pixels[col_byte * 8 + pixel_x][row];
                    if fb_byte.wrapping_shr(7 - pixel_x as u32) & 1 == 1 {
                        pixel.colored = true;
                    } else {
                        pixel.colored = false;
                    }
                }
            }
        }

        for pixel_row in pixels {
            for pixel in pixel_row {
                if pixel.colored {
                    canvas.set_draw_color(Color::RGB(0, 0xcc, 0x11));
                    canvas.fill_rect(pixel.rect).unwrap();
                }
            }
        }

        // Change KEY_PRESSED key states to KEY_DOWN.
        // TODO: Describe the trick.
        for i in 0..state.key_states.len() {
            state.key_states[i] |= 1;
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    match keycode {
                        Keycode::P => {
                            step_mode_active = !step_mode_active;
                            if step_mode_active {
                                next_instruction = false;
                            }
                        }
                        Keycode::N => {
                            // If step mode is not active, always move to the next instruction. Otherwise,
                            // only do it when the key is pressed.
                            next_instruction = !step_mode_active || !next_instruction;
                        }
                        Keycode::L => {
                            print_state(&state);
                        }
                        // RELEASED
                        // PRESSED
                        // DOWN
                        //
                        // 00 -> 01 -> 11
                        // f(RELEASED) -> PRESSED
                        // f(PRESSED) -> DOWN
                        // ^ 110 >> 1
                        // 000 -> 011
                        // 011 -> 010
                        // 010 -> 010

                        // 0 -> 0
                        // 1 -> 2
                        // 2 -> 2
                        //
                        // 11 | 01 = 11 KEY_UP
                        // 00 | 01 = 01 KEY_PRESSED
                        // 01 | 01 = 01 KEY_DOWN
                        //
                        Keycode::Num1 => state.key_states[0x1] = KEY_PRESSED,
                        Keycode::Num2 => state.key_states[0x2] = KEY_PRESSED,
                        Keycode::Num3 => state.key_states[0x3] = KEY_PRESSED,
                        Keycode::Num4 => state.key_states[0xc] = KEY_PRESSED,
                        Keycode::Q => state.key_states[0x4] = KEY_PRESSED,
                        Keycode::W => state.key_states[0x5] = KEY_PRESSED,
                        Keycode::E => state.key_states[0x6] = KEY_PRESSED,
                        Keycode::R => state.key_states[0xd] = KEY_PRESSED,
                        Keycode::A => state.key_states[0x7] = KEY_PRESSED,
                        Keycode::S => state.key_states[0x8] = KEY_PRESSED,
                        Keycode::D => state.key_states[0x9] = KEY_PRESSED,
                        Keycode::F => state.key_states[0xe] = KEY_PRESSED,
                        Keycode::Z => state.key_states[0xa] = KEY_PRESSED,
                        Keycode::X => state.key_states[0x0] = KEY_PRESSED,
                        Keycode::C => state.key_states[0xb] = KEY_PRESSED,
                        Keycode::V => state.key_states[0xf] = KEY_PRESSED,
                        _ => (),
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Num1 => state.key_states[0x1] = KEY_UP,
                    Keycode::Num2 => state.key_states[0x2] = KEY_UP,
                    Keycode::Num3 => state.key_states[0x3] = KEY_UP,
                    Keycode::Num4 => state.key_states[0xc] = KEY_UP,
                    Keycode::Q => state.key_states[0x4] = KEY_UP,
                    Keycode::W => state.key_states[0x5] = KEY_UP,
                    Keycode::E => state.key_states[0x6] = KEY_UP,
                    Keycode::R => state.key_states[0xd] = KEY_UP,
                    Keycode::A => state.key_states[0x7] = KEY_UP,
                    Keycode::S => state.key_states[0x8] = KEY_UP,
                    Keycode::D => state.key_states[0x9] = KEY_UP,
                    Keycode::F => state.key_states[0xe] = KEY_UP,
                    Keycode::Z => state.key_states[0xa] = KEY_UP,
                    Keycode::X => state.key_states[0x0] = KEY_UP,
                    Keycode::C => state.key_states[0xb] = KEY_UP,
                    Keycode::V => state.key_states[0xf] = KEY_UP,
                    _ => (),
                },
                _ => {}
            }
        }

        if next_instruction {
            // update timer
            if state.delay_timer > 0 {
                state.delay_timer -= 1;
            }

            if state.sound_timer == 0 {
                audio_device.pause();
            } else {
                if audio_device.status() != AudioStatus::Playing {
                    audio_device.resume();
                }
                state.sound_timer -= 1;
            }

            let opcode_address = state.program_counter as usize;
            let opcode: u16 = ((state.memory[opcode_address] as u16) << 8)
                | (state.memory[opcode_address + 1] as u16);
            let instruction = decode_opcode(opcode);
            if true {
                //            if step_mode_active {
                println!(
                    "{}: 0x{:04X} => {:?}",
                    state.program_counter, opcode, instruction
                );
            }
            if let Instruction::INVALID = instruction {
                break;
            }
            execute_instruction(instruction, &mut state);
        }

        canvas.present();
        ::std::thread::sleep(Duration::from_micros(16666));
    }
}
