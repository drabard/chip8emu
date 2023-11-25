use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::EventPump;

#[derive(Clone, Copy)]
pub enum Key {
    Key0 = 0,
    Key1 = 1,
    Key2 = 2,
    Key3 = 3,
    Key4 = 4,
    Key5 = 5,
    Key6 = 6,
    Key7 = 7,
    Key8 = 8,
    Key9 = 9,
    KeyA = 0xa,
    KeyB = 0xb,
    KeyC = 0xc,
    KeyD = 0xd,
    KeyE = 0xe,
    KeyF = 0xf,
}

impl From<u8> for Key {
    fn from(value: u8) -> Key {
        match value {
            0 => Key::Key0,
            1 => Key::Key1,
            2 => Key::Key2,
            3 => Key::Key3,
            4 => Key::Key4,
            5 => Key::Key5,
            6 => Key::Key6,
            7 => Key::Key7,
            8 => Key::Key8,
            9 => Key::Key9,
            0xa => Key::KeyA,
            0xb => Key::KeyB,
            0xc => Key::KeyC,
            0xd => Key::KeyD,
            0xe => Key::KeyE,
            0xf => Key::KeyF,
            _ => panic!("Invalid Key."),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum KeyState {
    KeyUp = 0b01,
    KeyPressed = 0b10,
    KeyDown = 0b11,
}

impl From<u8> for KeyState {
    fn from(value: u8) -> KeyState {
        match value {
            0b01 => KeyState::KeyUp,
            0b10 => KeyState::KeyPressed,
            0b11 => KeyState::KeyDown,
            _ => panic!("Invalid KeyState value."),
        }
    }
}

pub struct Input {
    chip8_keys: [KeyState; 0x10],
    pub quit: bool,
    pub step_mode_changed: bool,
    pub step_to_next_instruction: bool,
    pub print_state: bool,
}

impl Input {
    pub fn new() -> Input {
        Input {
            chip8_keys: [KeyState::KeyUp; 0x10],
            quit: false,
            step_mode_changed: false,
            step_to_next_instruction: false,
            print_state: false,
        }
    }

    pub fn collect(self: &mut Self, event_pump: &mut EventPump) {
        for i in 0..self.chip8_keys.len() {
            self.chip8_keys[i] = KeyState::from(self.chip8_keys[i] as u8 | 1 as u8);
        }

        self.quit = false;
        self.step_mode_changed = false;
        self.step_to_next_instruction = false;
        self.print_state = false;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => self.quit = true,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::P => self.step_mode_changed = true,
                    Keycode::N => self.step_to_next_instruction = true,
                    Keycode::L => self.print_state = true,
                    Keycode::Num1 => self.chip8_keys[0x1] = KeyState::KeyPressed,
                    Keycode::Num2 => self.chip8_keys[0x2] = KeyState::KeyPressed,
                    Keycode::Num3 => self.chip8_keys[0x3] = KeyState::KeyPressed,
                    Keycode::Num4 => self.chip8_keys[0xc] = KeyState::KeyPressed,
                    Keycode::Q => self.chip8_keys[0x4] = KeyState::KeyPressed,
                    Keycode::W => self.chip8_keys[0x5] = KeyState::KeyPressed,
                    Keycode::E => self.chip8_keys[0x6] = KeyState::KeyPressed,
                    Keycode::R => self.chip8_keys[0xd] = KeyState::KeyPressed,
                    Keycode::A => self.chip8_keys[0x7] = KeyState::KeyPressed,
                    Keycode::S => self.chip8_keys[0x8] = KeyState::KeyPressed,
                    Keycode::D => self.chip8_keys[0x9] = KeyState::KeyPressed,
                    Keycode::F => self.chip8_keys[0xe] = KeyState::KeyPressed,
                    Keycode::Z => self.chip8_keys[0xa] = KeyState::KeyPressed,
                    Keycode::X => self.chip8_keys[0x0] = KeyState::KeyPressed,
                    Keycode::C => self.chip8_keys[0xb] = KeyState::KeyPressed,
                    Keycode::V => self.chip8_keys[0xf] = KeyState::KeyPressed,
                    _ => (),
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Num1 => self.chip8_keys[0x1] = KeyState::KeyUp,
                    Keycode::Num2 => self.chip8_keys[0x2] = KeyState::KeyUp,
                    Keycode::Num3 => self.chip8_keys[0x3] = KeyState::KeyUp,
                    Keycode::Num4 => self.chip8_keys[0xc] = KeyState::KeyUp,
                    Keycode::Q => self.chip8_keys[0x4] = KeyState::KeyUp,
                    Keycode::W => self.chip8_keys[0x5] = KeyState::KeyUp,
                    Keycode::E => self.chip8_keys[0x6] = KeyState::KeyUp,
                    Keycode::R => self.chip8_keys[0xd] = KeyState::KeyUp,
                    Keycode::A => self.chip8_keys[0x7] = KeyState::KeyUp,
                    Keycode::S => self.chip8_keys[0x8] = KeyState::KeyUp,
                    Keycode::D => self.chip8_keys[0x9] = KeyState::KeyUp,
                    Keycode::F => self.chip8_keys[0xe] = KeyState::KeyUp,
                    Keycode::Z => self.chip8_keys[0xa] = KeyState::KeyUp,
                    Keycode::X => self.chip8_keys[0x0] = KeyState::KeyUp,
                    Keycode::C => self.chip8_keys[0xb] = KeyState::KeyUp,
                    Keycode::V => self.chip8_keys[0xf] = KeyState::KeyUp,
                    _ => (),
                },
                _ => {}
            }
        }
    }

    pub fn any_key_pressed(self: &Self) -> Option<Key> {
        match self
            .chip8_keys
            .iter()
            .position(|&k| k == KeyState::KeyPressed)
        {
            Some(idx) => Some(Key::from(idx as u8)),
            None => None,
        }
    }

    pub fn get_key_state(self: &Self, key: Key) -> KeyState {
        return self.chip8_keys[key as usize];
    }
}
