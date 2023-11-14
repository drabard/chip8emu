use std::fs::File;
use std::env;
use std::io::{self, Read};

struct State {
    memory: [u8; 0xfff],
    registers: [u8; 0xf],
    stack: [u16; 0xf],
    instruction_pointer: u16,
    delay_timer: u8,
    sound_timer: u8,
    // internal registers (not available for CHIP-8 programs)
    program_counter: u16,
    stack_pointer: u8,
}

impl State {
    fn new() -> State {
        State {
            memory: [0; 0xfff],
            registers: [0; 0xf],
            stack: [0; 0xf],
            instruction_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            program_counter: 0,
            stack_pointer: 0,
        }
    }
}

#[derive(Debug)]
enum Instruction {
    // For descriptions, see http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
    INVALID,
    SYS {
        address: u16,
    },
    CLS,
    RET,
    JP {
        address: u16,
    },
    CALL {
        address: u16,
    },
    SERV {
        register: u8,
        value: u8,
    },
    SNERV {
        register: u8,
        value: u8,
    },
    SERR {
        register0: u8,
        register1: u8,
    },
    LDRV {
        register: u8,
        value: u8,
    },
    ADDRV {
        register: u8,
        value: u8,
    },
    LDRR {
        register0: u8,
        register1: u8,
    },
    ORRR {
        register0: u8,
        register1: u8,
    },
    ANDRR {
        register0: u8,
        register1: u8,
    },
    XORRR {
        register0: u8,
        register1: u8,
    },
    ADDRR {
        register0: u8,
        register1: u8,
    },
    SUBRR {
        register0: u8,
        register1: u8,
    },
    SHR {
        register0: u8,
        register1: u8,
    },
    SUBN {
        register0: u8,
        register1: u8,
    },
    SHL {
        register0: u8,
        register1: u8,
    },
    SNERR {
        register0: u8,
        register1: u8,
    },
    LDI {
        address: u16,
    },
    JP0A {
        address: u16,
    },
    RND {
        register: u8,
        value: u8,
    },
    DRW {
        register0: u8,
        register1: u8,
        nibble: u8,
    },
    SKP {
        register: u8,
    },
    SKNP {
        register: u8,
    },
    LDRDT {
        register: u8,
    },
    LDRK {
        register: u8,
    },
    LDDTR {
        register: u8,
    },
    LDSTR {
        register: u8,
    },
    ADDI {
        register: u8,
    },
    LDF {
        register: u8,
    },
    LDB {
        register: u8,
    },
    LDIR {
        register: u8,
    },
    LDRI {
        register: u8,
    },
}

fn decode_opcode(opcode: u16) -> Instruction {
    match (opcode & 0xf000) >> 12 {
        0 => match opcode & 0xfff {
            0x0E0 => Instruction::CLS,
            0x0EE => Instruction::RET,
            _ => Instruction::INVALID
        },
        1 => Instruction::JP {
            address: opcode & 0xfff,
        },
        2 => Instruction::CALL {
            address: opcode & 0xfff,
        },
        3 => Instruction::SERV {
            register: ((opcode & 0xf00) >> 8) as u8,
            value: (opcode & 0xff) as u8,
        },
        4 => Instruction::SNERV {
            register: ((opcode & 0xf00) >> 8) as u8,
            value: (opcode & 0xff) as u8,
        },
        5 => Instruction::SERR {
            register0: ((opcode & 0xf00) >> 8) as u8,
            register1: ((opcode & 0xf0) >> 4) as u8,
        },
        6 => Instruction::LDRV {
            register: ((opcode & 0xf00) >> 8) as u8,
            value: (opcode & 0xff) as u8,
        },
        7 => Instruction::ADDRV {
            register: ((opcode & 0xf00) >> 8) as u8,
            value: (opcode & 0xff) as u8,
        },
        8 => match opcode & 0xf {
            0 => Instruction::LDRR {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            1 => Instruction::ORRR {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            2 => Instruction::ANDRR {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            3 => Instruction::XORRR {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            4 => Instruction::ADDRR {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            5 => Instruction::SUBRR {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            6 => Instruction::SHR {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            7 => Instruction::SUBN {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            0xe => Instruction::SHL {
                register0: ((opcode & 0xf00) >> 8) as u8,
                register1: ((opcode & 0xf0) >> 4) as u8,
            },
            _ => Instruction::INVALID
        },
        9 => Instruction::SNERR {
            register0: ((opcode & 0xf00) >> 8) as u8,
            register1: ((opcode & 0xf0) >> 4) as u8,
        },
        0xa => Instruction::LDI {
            address: opcode & 0xfff,
        },
        0xb => Instruction::JP0A {
            address: opcode & 0xfff,
        },
        0xc => Instruction::RND {
            register: ((opcode & 0xf00) >> 8) as u8,
            value: (opcode & 0xff) as u8,
        },
        0xd => Instruction::DRW {
            register0: ((opcode & 0xf00) >> 8) as u8,
            register1: ((opcode & 0xf0) >> 4) as u8,
            nibble: (opcode & 0xf) as u8,
        },
        0xe => match opcode & 0xff {
            0x9E => Instruction::SKP {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0xA1 => Instruction::SKNP {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            _ => Instruction::INVALID
        },
        0xf => match opcode & 0xff {
            0x07 => Instruction::LDRDT {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0x0A => Instruction::LDRK {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0x15 => Instruction::LDDTR {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0x18 => Instruction::LDSTR {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0x1E => Instruction::ADDI {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0x29 => Instruction::LDF {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0x33 => Instruction::LDB {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0x55 => Instruction::LDIR {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            0x65 => Instruction::LDRI {
                register: ((opcode & 0xf00) >> 8) as u8,
            },
            _ => panic!("Invalid opcode: {:?}", opcode),
        },
        _ => Instruction::INVALID
    }
}

fn main() {
    let mut state = State::new();

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Invalid usage: provide path to CHIP-8 ROM as argument.");
    }

    let mut opcode_buffer: Vec<u8> = Vec::new();
    let file_path = args[1].as_str();
    let file = File::open(file_path).unwrap();
    file.take(0xffff).read_to_end(&mut opcode_buffer).unwrap();

    for i in (3..opcode_buffer.len()).step_by(2) {
        // println!("0x{:X}, 0x{:X}", opcode_buffer[i], opcode_buffer[i + 1]);
        let opcode: u16 = ((opcode_buffer[i] as u16) << 8) | (opcode_buffer[i + 1] as u16);
        let instruction = decode_opcode(opcode);
        println!("{}: 0x{:X} => {:?}", i - 3, opcode, instruction);
    }

    println!("Hello, world!");
}
