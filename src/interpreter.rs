use rand::{rngs::ThreadRng, Rng};

use crate::input::{Input, Key, KeyState};
use crate::sound::Sound;
use crate::display::Display;

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

#[derive(Debug)]
pub struct Interpreter {
    framebuffer: [u8; 256],
    memory: [u8; 0xfff],
    registers: [u8; 16],
    stack: [u16; 0xf],
    memory_register: u16,
    delay_timer: u8,
    sound_timer: u8,
    // internal registers (not available for CHIP-8 programs)
    program_counter: u16,
    stack_pointer: usize,
    random_number_generator: ThreadRng,
    previous_status: ExecutionStatus
}

#[derive(Debug, Clone, Copy)]
pub enum ExecutionStatus {
    Ok,
    FramebufferChanged
}

type Address = u16;
type Register = usize;
type Value = u8;
type Nibble = u8;

impl Interpreter {
    pub fn new(rom_buffer: &[u8]) -> Interpreter {
        let mut memory = [0; 0xfff];

        // Initialize hard-coded digit sprites. These should reside in the interpreter
        // area of memory - 0x000 to 0x1ff. We put them right at the beginning, at addresses 0x0000 - 0x0050.
        memory[0..5].copy_from_slice(&[0xf0, 0x90, 0x90, 0x90, 0xf0]); // 0
        memory[5..10].copy_from_slice(&[0x20, 0x60, 0x20, 0x20, 0x70]); // 1
        memory[10..15].copy_from_slice(&[0xf0, 0x10, 0xf0, 0x80, 0xf0]); // 2
        memory[15..20].copy_from_slice(&[0xf0, 0x10, 0xf0, 0x10, 0xf0]); // 3
        memory[20..25].copy_from_slice(&[0x90, 0x90, 0xf0, 0x10, 0x10]); // 4
        memory[25..30].copy_from_slice(&[0xf0, 0x80, 0xf0, 0x10, 0xf0]); // 5
        memory[30..35].copy_from_slice(&[0xf0, 0x80, 0xf0, 0x90, 0xf0]); // 6
        memory[35..40].copy_from_slice(&[0xf0, 0x10, 0x20, 0x40, 0x40]); // 7
        memory[40..45].copy_from_slice(&[0xf0, 0x90, 0xf0, 0x90, 0xf0]); // 8
        memory[45..50].copy_from_slice(&[0xf0, 0x90, 0xf0, 0x10, 0xf0]); // 9
        memory[50..55].copy_from_slice(&[0xf0, 0x90, 0xf0, 0x90, 0x90]); // A
        memory[55..60].copy_from_slice(&[0xe0, 0x90, 0xe0, 0x90, 0xe0]); // B
        memory[60..65].copy_from_slice(&[0xf0, 0x80, 0x80, 0x80, 0xf0]); // C
        memory[65..70].copy_from_slice(&[0xe0, 0x90, 0x90, 0x90, 0xe0]); // D
        memory[70..75].copy_from_slice(&[0xf0, 0x80, 0xf0, 0x80, 0xf0]); // E
        memory[75..80].copy_from_slice(&[0xf0, 0x80, 0xf0, 0x80, 0x80]); // F

        let mut interpreter = Interpreter {
            framebuffer: [0; 256],
            memory,
            registers: [0; 16],
            stack: [0; 0xf],
            memory_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            program_counter: 0,
            stack_pointer: 0,
            random_number_generator: rand::thread_rng(),
            previous_status: ExecutionStatus::Ok
        };

        let magic_string = match std::str::from_utf8(&rom_buffer[0..3]) {
            Ok(magic_string) => magic_string,
            Err(..) => "",
        };
        if magic_string == "C8P" {
            interpreter.memory[0x200..0x200 + (rom_buffer.len() - 3)]
                .copy_from_slice(&rom_buffer[3..]);
        } else {
            interpreter.memory[0x200..0x200 + (rom_buffer.len())].copy_from_slice(&rom_buffer);
        }
        interpreter.program_counter = 0x200;

        interpreter
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
                _ => Instruction::INVALID,
            },
            _ => Instruction::INVALID,
        }
    }

    fn execute_instruction(self: &mut Interpreter, instruction: Instruction, input: &Input) -> ExecutionStatus {
        self.program_counter += 2;

        let mut status = ExecutionStatus::Ok;

        match instruction {
            Instruction::INVALID => (),
            Instruction::SYS => (),
            Instruction::CLS => {
                self.framebuffer = [0; 256];
            }
            Instruction::RET => {
                self.program_counter = self.stack[self.stack_pointer];
                self.stack_pointer -= 1;
            }
            Instruction::JP(address) => {
                self.program_counter = address;
            }
            Instruction::CALL(address) => {
                self.stack_pointer += 1;
                self.stack[self.stack_pointer] = self.program_counter;
                self.program_counter = address;
            }
            Instruction::SERV(register, value) => {
                if self.registers[register] == value {
                    self.program_counter += 2;
                }
            }
            Instruction::SNERV(register, value) => {
                if self.registers[register] != value {
                    self.program_counter += 2;
                }
            }
            Instruction::SERR(register0, register1) => {
                if self.registers[register0] == self.registers[register1] {
                    self.program_counter += 2;
                }
            }
            Instruction::LDRV(register, value) => {
                self.registers[register] = value;
            }
            Instruction::ADDRV(register, value) => {
                let sum: u16 = self.registers[register] as u16 + value as u16;
                self.registers[0xf] = if sum > 255 { 1 } else { 0 };
                self.registers[register] = self.registers[register].wrapping_add(value);
            }
            Instruction::LDRR(register0, register1) => {
                self.registers[register0] = self.registers[register1];
            }
            Instruction::ORRR(register0, register1) => {
                self.registers[register0] |= self.registers[register1];
            }
            Instruction::ANDRR(register0, register1) => {
                self.registers[register0] &= self.registers[register1];
            }
            Instruction::XORRR(register0, register1) => {
                self.registers[register0] ^= self.registers[register1];
            }
            Instruction::ADDRR(register0, register1) => {
                let sum: u16 = self.registers[register1] as u16 + self.registers[register0] as u16;
                self.registers[0xf] = if sum > 255 { 1 } else { 0 };
                self.registers[register0] =
                    self.registers[register0].wrapping_add(self.registers[register1]);
            }
            Instruction::SUBRR(register0, register1) => {
                let diff: i16 = self.registers[register0] as i16 - self.registers[register1] as i16;
                self.registers[0xf] = if diff > 0 { 1 } else { 0 };
                self.registers[register0] =
                    self.registers[register0].wrapping_sub(self.registers[register1]);
            }
            Instruction::SHR(register0, _) => {
                self.registers[0xf] = if self.registers[register0] & 1 == 1 {
                    1
                } else {
                    0
                };
                self.registers[register0] >>= 1;
            }
            Instruction::SUBN(register0, register1) => {
                let diff: i16 = self.registers[register1] as i16 - self.registers[register0] as i16;
                self.registers[0xf] = if diff > 0 { 1 } else { 0 };
                self.registers[register0] =
                    self.registers[register1].wrapping_sub(self.registers[register0]);
            }
            Instruction::SHL(register0, _register1) => {
                self.registers[0xf] = if self.registers[register0] & 0x80 == 0x80 {
                    1
                } else {
                    0
                };
                self.registers[register0] <<= 1;
            }
            Instruction::SNERR(register0, register1) => {
                if self.registers[register0] != self.registers[register1] {
                    self.program_counter += 2;
                }
            }
            Instruction::LDI(address) => {
                self.memory_register = address;
            }
            Instruction::JP0A(address) => {
                self.program_counter = address + self.registers[0] as u16;
            }
            Instruction::RND(register, value) => {
                let random_number = self.random_number_generator.gen_range(0..=255);
                self.registers[register] = random_number & value;
            }
            Instruction::DRW(register0, register1, nibble) => {
                let screen_x = self.registers[register0] as usize;
                for row in 0..nibble {
                    let screen_y = (self.registers[register1] + row) as usize;
                    let bit_offset = screen_x % 8;

                    let sprite_byte = self.memory[(self.memory_register + row as u16) as usize];
                    let sprite_bits: u16 = (sprite_byte as u16) << (8 - bit_offset);

                    let fb_byte_idx = (screen_x / 8 + screen_y * 8) % 256;
                    self.framebuffer[fb_byte_idx] ^= (sprite_bits >> 8) as u8;
                    if fb_byte_idx == self.framebuffer.len() - 1 {
                        self.framebuffer[0] ^= sprite_bits as u8;
                    } else {
                        self.framebuffer[fb_byte_idx + 1] ^= sprite_bits as u8;
                    }
                }

                status = ExecutionStatus::FramebufferChanged;
            }
            Instruction::SKP(register) => {
                if input.get_key_state(Key::from(self.registers[register])) != KeyState::KeyUp {
                    self.program_counter += 2;
                }
            }
            Instruction::SKNP(register) => {
                if input.get_key_state(Key::from(self.registers[register])) == KeyState::KeyUp {
                    self.program_counter += 2;
                }
            }
            Instruction::LDRDT(register) => {
                self.registers[register] = self.delay_timer;
            }
            Instruction::LDRK(register) => match input.any_key_pressed() {
                Some(key) => self.registers[register as usize] = key as u8,
                None => self.program_counter -= 2,
            },
            Instruction::LDDTR(register) => {
                self.delay_timer = self.registers[register];
            }
            Instruction::LDSTR(register) => {
                self.sound_timer = self.registers[register];
            }
            Instruction::ADDI(register) => {
                self.memory_register = self
                    .memory_register
                    .wrapping_add(self.registers[register] as u16);
            }
            Instruction::LDF(register) => {
                self.memory_register = (self.registers[register] * 5) as u16;
            }
            Instruction::LDB(register) => {
                let value: f32 = self.registers[register] as f32;
                let hundreds = (value / 100.0).floor();
                let tens = ((value - hundreds * 100.0) / 10.0).floor();
                let ones = (value - hundreds * 100.0 - tens * 10.0).floor();
                self.memory[self.memory_register as usize] = hundreds as u8;
                self.memory[self.memory_register as usize + 1] = tens as u8;
                self.memory[self.memory_register as usize + 2] = ones as u8;
            }
            Instruction::LDIR(register) => {
                let num_registers = register + 1 as usize;
                let mem_start = self.memory_register as usize;
                let mem_end = (mem_start + num_registers) as usize;
                self.memory[mem_start..mem_end].copy_from_slice(&self.registers[0..num_registers]);
            }
            Instruction::LDRI(register) => {
                let num_registers = register + 1 as usize;
                let mem_start = self.memory_register as usize;
                let mem_end = (mem_start + num_registers) as usize;
                self.registers[0..num_registers as usize]
                    .copy_from_slice(&self.memory[mem_start..mem_end]);
            }
        }

        status
    }

    pub fn execute_next_instruction(self: &mut Self, display: &mut Display, sound: &mut Sound, input: &Input) -> Result<ExecutionStatus, String> {
        match self.previous_status {
            ExecutionStatus::FramebufferChanged => display.set_pixels(&self.framebuffer),
            _ => ()
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer == 0 {
            sound.stop();
        } else {
            sound.play();
            self.sound_timer -= 1;
        }

        let opcode_address = self.program_counter as usize;
        let opcode: u16 =
            ((self.memory[opcode_address] as u16) << 8) | (self.memory[opcode_address + 1] as u16);
        let instruction = Interpreter::decode_opcode(opcode);
        if true {
            println!(
                "{}: 0x{:04X} => {:?}",
                self.program_counter, opcode, instruction
            );
        }

        if let Instruction::INVALID = instruction {
            return Err("Invalid instruction.".to_string());
        }

        self.previous_status = self.execute_instruction(instruction, input);

        Ok(self.previous_status.clone())
    }

    pub fn print_state(self: &Self) {
        print!(
            "=================
Registers: {:?}
Memory register: {:?}
Stack: {:?}
Delay timer: {}
=================
",
            self.registers, self.memory_register, self.stack, self.delay_timer
        );

        println!("Memory at memory register:");
        for i in 0..32 {
            print!("{:3X}: ", self.memory_register + i * 16);
            for j in 0..16 {
                print!(
                    "{:2X} ",
                    self.memory[(self.memory_register + j + i * 16) as usize]
                );
            }
            print!("\n");
        }
    }
}
