use std::fs::File;
use std::io::prelude::*;

use rand::Rng;

const chip8_fontset: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, //0
    0x20, 0x60, 0x20, 0x20, 0x70, //1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, //2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, //3
    0x90, 0x90, 0xF0, 0x10, 0x10, //4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, //5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, //6
    0xF0, 0x10, 0x20, 0x40, 0x40, //7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, //8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, //9
    0xF0, 0x90, 0xF0, 0x90, 0x90, //A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, //B
    0xF0, 0x80, 0x80, 0x80, 0xF0, //C
    0xE0, 0x90, 0x90, 0x90, 0xE0, //D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, //E
    0xF0, 0x80, 0xF0, 0x80, 0x80, //F
];

pub struct Chip8 {
    opcode: u16,
    pc: usize,
    index: usize,

    stack: [u16; 16],
    sp: usize,

    registers: [u8; 16],
    memory: Box<[u8]>,

    sound_timer: u8,
    delay_timer: u8,

    rng: rand::rngs::ThreadRng,

    pub gfx: Vec<Vec<u8>>,
    pub keypad: [u8; 16],
    pub draw_flag: bool,
}

impl Chip8 {
    pub fn new() -> Self {
        let mut memory: [u8; 4096] = [0; 4096];
        for i in 0..80 {
            memory[i] = chip8_fontset[i];
        }

        Self {
            opcode: 0,
            pc: 0x200,
            index: 0,

            stack: [0; 16],
            sp: 0,

            registers: [0; 16],
            memory: Box::new(memory),

            delay_timer: 0,
            sound_timer: 0,

            // gfx: Box::new([Box::new([0; 2048]); 2048]),
            gfx: vec![vec!(0; 2048); 2048],
            keypad: [0; 16],
            draw_flag: false,

            rng: rand::thread_rng(),
        }
    }

    pub fn load(&mut self, rom_path: &str) -> bool {
        eprintln!("Loading ROM: {}", rom_path);

        let rom = File::open(rom_path).expect("Failed to open rom!");
        let rom_size = rom.metadata().expect("Failed to fetch metadata!").len();
        println!("rom size: {}", rom_size);

        let mut rom_buffer: Vec<u8> = vec![];
        for x in rom.bytes() {
            if let Ok(y) = x {
                rom_buffer.push(y);
            }
        }

        if (4096 - 512) > rom_size {
            for i in 0..rom_size {
                self.memory[(i + 512) as usize] = rom_buffer[i as usize] as u8;
            }
            println!("rom successfully loaded from buffer to 4k wram");
        } else {
            eprintln!("ROM too large to fit in memory!");
            return false;
        }

        true
    }

    pub fn emulate_cycle(&mut self) {
        self.opcode = (self.memory[self.pc] as u16) << 8 | self.memory[self.pc + 1] as u16;
        eprintln!("emulating cycle... {:X?}", self.opcode);
        match self.opcode & 0xF000 {
            0x0 => {
                eprintln!("0x0...");
                match self.opcode & 0x000F {
                    // clear screen
                    0x0 => {
                        for y in 0..2048 {
                            for x in 0..2048 {
                                self.gfx[y][x] = 0;
                            }
                        }

                        self.draw_flag = true;
                    }

                    // return from subroutine
                    0xE => {
                        self.sp -= 1;
                        self.pc = self.stack[self.sp] as usize;
                    }

                    x => panic!("unexpected instr at 0: {}", x),
                };

                self.pc += 2;
            }

            // 0x1NNN - jump to address NNN
            0x1000 => self.pc = (self.opcode & 0x0FFF) as usize,

            // 0x2NNN - calls subroutine at NNN
            0x2000 => {
                self.stack[self.sp] = self.pc as u16;
                self.sp += 1;
                self.pc = (self.opcode & 0x0FFF) as usize;
            }

            // 3XNN - Skips the next instruction if reg[X] equals NN.
            0x3000 => {
                if self.registers[((self.opcode & 0x0F00) >> 8) as usize]
                    == (self.opcode & 0x00FF) as u8
                {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }

            // 4XNN - Skips the next instruction if VX does not equal NN.
            0x4000 => {
                if self.registers[((self.opcode & 0x0F00) >> 8) as usize]
                    != (self.opcode & 0x00FF) as u8
                {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }

            // 5XY0 - Skips the next instruction if VX equals VY.
            0x5000 => {
                if self.registers[((self.opcode & 0x0F00) >> 8) as usize]
                    == self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }

            // 6XNN - Sets VX to NN.
            0x6000 => {
                self.registers[((self.opcode & 0xF00) >> 8) as usize] =
                    (self.opcode & 0x00FF) as u8;
                self.pc += 2;
            }

            // 7XNN - Adds NN to VX.
            0x7000 => {
                self.registers[((self.opcode & 0xF00) >> 8) as usize] +=
                    (self.opcode & 0x00FF) as u8;
                self.pc += 2;
            }

            // 8XY_ - Data processing
            0x8000 => {
                match self.opcode & 0x000F {
                    // 8XY0 - Sets VX to the value of VY.
                    0 => {
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] =
                            self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                    }
                    // 8XY1 - Sets VX to (VX OR VY).
                    1 => {
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] |=
                            self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                    }
                    // 8XY2 - Sets VX to (VX AND VY).
                    2 => {
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] &=
                            self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                    }
                    // 8XY3 - Sets VX to (VX XOR VY).
                    3 => {
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] ^=
                            self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                    }

                    // 8XY4 - Adds VY to VX. VF is set to 1 when there's a carry,
                    // and to 0 when there isn't.
                    4 => {
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] +=
                            self.registers[((self.opcode & 0x00F0) >> 4) as usize];

                        if self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                            > (0xFF - self.registers[((self.opcode & 0x0F00) >> 8) as usize])
                        {
                            self.registers[0xF] = 1; // carry
                        } else {
                            self.registers[0xF] = 0;
                        }
                    }

                    // 8XY5 - VY is subtracted from VX. VF is set to 0 when
                    // there's a borrow, and 1 when there isn't.
                    5 => {
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] -=
                            self.registers[((self.opcode & 0x00F0) >> 4) as usize];

                        if self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                            > self.registers[((self.opcode & 0x0F00) >> 8) as usize]
                        {
                            self.registers[0xF] = 0; // there is a borrow
                        } else {
                            self.registers[0xF] = 1;
                        }
                    }

                    // 0x8XY6 - Shifts VX right by one. VF is set to the value of
                    // the least significant bit of VX before the shift.
                    6 => {
                        self.registers[0xF] =
                            self.registers[((self.opcode & 0x0F00) >> 8) as usize] & 0x1;

                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] >>= 1;
                    }

                    // 0x8XY7: Sets VX to VY minus VX. VF is set to 0 when there's
                    // a borrow, and 1 when there isn't.
                    7 => {
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] = self.registers
                            [((self.opcode & 0x00F0) >> 4) as usize]
                            - self.registers[((self.opcode & 0x0F00) >> 8) as usize];

                        if self.registers[((self.opcode & 0x0F00) >> 8) as usize]
                            > self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                        {
                            self.registers[0xF] = 0; // there is a borrow
                        } else {
                            self.registers[0xF] = 1;
                        }
                    }

                    // 0x8XYE: Shifts VX left by one. VF is set to the value of
                    // the most significant bit of VX before the shift.
                    0xE => {
                        self.registers[0xF] =
                            self.registers[((self.opcode & 0x0F00) >> 8) as usize] >> 7;
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] <<= 1;
                    }

                    x => panic!("unknown data processing instr: {}", x),
                }
                self.pc += 2;
            }

            // 9XY0 - Skips the next instruction if VX doesn't equal VY.
            0x9000 => {
                if self.registers[((self.opcode & 0x0F00) >> 8) as usize]
                    != self.registers[((self.opcode & 0x00F0) >> 4) as usize]
                {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }

            // ANNN - Sets I to the address NNN.
            0xA000 => {
                self.index = (self.opcode & 0x0FFF) as usize;
                self.pc += 2;
            }

            // BNNN - Jumps to the address NNN plus V0.
            0xB000 => {
                self.pc = ((self.opcode & 0x0FFF) as u8 + self.registers[0]) as usize;
            }

            // CXNN - Sets VX to a random number, masked by NN.
            0xC000 => {
                self.registers[((self.opcode & 0x0F00) >> 8) as usize] =
                    ((self.rng.gen::<u16>() % (0xFF + 1)) & (self.opcode & 0x00FF)) as u8;
                self.pc += 2;
            }

            // DXYN: Draws a sprite at coordinate (VX, VY) that has a width of 8
            // pixels and a height of N pixels.
            // Each row of 8 pixels is read as bit-coded starting from memory
            // location I;
            // I value doesn't change after the execution of this instruction.
            // VF is set to 1 if any screen pixels are flipped from set to unset
            // when the sprite is drawn, and to 0 if that doesn't happen.
            0xD000 => {
                let x = self.registers[((self.opcode & 0x0F00) >> 8) as usize] as u16;
                let y = self.registers[((self.opcode & 0x00F0) >> 4) as usize] as u16;
                let height = self.opcode & 0x000F;
                let mut pixel;

                self.registers[0xF] = 0;

                for yline in 0..height {
                    pixel = self.memory[self.index + yline as usize];
                    for xline in 0..8 {
                        if (pixel & (0x80 >> xline)) != 0 {
                            if self.gfx[((y as u16 + yline) * 64) as usize][(x + xline) as usize]
                                == 1
                            {
                                self.registers[0xF] = 1;
                            }
                            self.gfx[((y + yline) * 64) as usize][(x + xline) as usize] ^= 1;
                            eprintln!("xline: {}", xline);
                        }
                    }
                }
                eprintln!("exited D000");

                self.draw_flag = true;
                self.pc += 2;
            }

            0xE000 => {
                match self.opcode & 0x00FF {
                    // EX9E - Skips the next instruction if the key stored
                    // in VX is pressed.
                    0x9E => {
                        if self.keypad
                            [self.registers[((self.opcode & 0x0F00) >> 8) as usize] as usize]
                            != 0
                        {
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }

                    // EXA1 - Skips the next instruction if the key stored
                    // in VX isn't pressed.
                    0xA1 => {
                        if self.keypad
                            [self.registers[((self.opcode & 0x0F00) >> 8) as usize] as usize]
                            == 0
                        {
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }

                    x => panic!("unexpected instr in keypad: {}", x),
                }
            }

            0xF000 => {
                match self.opcode & 0x00FF {
                    // FX07 - Sets VX to the value of the delay timer
                    0x07 => {
                        self.registers[((self.opcode & 0x0F00) >> 8) as usize] = self.delay_timer;
                        self.pc += 2;
                    }

                    // FX0A - A key press is awaited, and then stored in VX
                    0x0A => {
                        let mut key_pressed = false;
                        for i in 0..16 {
                            if self.keypad[i] != 0 {
                                self.registers[((self.opcode & 0x0F00) >> 8) as usize] = i as u8;
                                key_pressed = true;
                            }
                        }

                        if !key_pressed {
                            return;
                        }

                        self.pc += 2;
                    }

                    // FX15 - Sets the delay timer to VX
                    0x15 => {
                        self.delay_timer = self.registers[((self.opcode & 0x0F00) >> 8) as usize];
                        self.pc += 2;
                    }

                    // FX1E - Adds VX to I
                    0x1E => {
                        if self.index as u16
                            + self.registers[((self.opcode & 0x0F00) >> 8) as usize] as u16
                            > 0xFFF
                        {
                            self.registers[0xF] = 1;
                        } else {
                            self.registers[0xF] = 0;
                        }

                        self.index +=
                            self.registers[((self.opcode & 0x0F00) >> 8) as usize] as usize;
                        self.pc += 2;
                    }

                    // FX29 - Sets I to the location of the sprite for the
                    // character in VX. Characters 0-F (in hexadecimal) are
                    // represented by a 4x5 font
                    0x29 => {
                        self.index =
                            self.registers[((self.opcode & 0x0F00) >> 8) as usize] as usize * 5;
                        self.pc += 2;
                    }

                    // FX33 - Stores the Binary-coded decimal representation of VX
                    // at the addresses I, I plus 1, and I plus 2
                    0x33 => {
                        self.memory[self.index] =
                            self.registers[((self.opcode & 0x0F00) >> 8) as usize] / 100;
                        self.memory[self.index + 1] =
                            (self.registers[((self.opcode & 0x0F00) >> 8) as usize] / 10) % 10;
                        self.memory[self.index + 2] =
                            self.registers[((self.opcode & 0x0F00) >> 8) as usize] % 10;
                        self.pc += 2;
                    }

                    // FX55 - Stores V0 to VX in memory starting at address I
                    0x55 => {
                        for i in 0..((self.opcode & 0x0F00) >> 8) {
                            self.memory[self.index as usize + i as usize] =
                                self.registers[i as usize] as u8;
                        }

                        self.index += (((self.opcode & 0x0F00) >> 8) + 1) as usize;
                        self.pc += 2;
                    }

                    0x65 => {
                        for i in 0..((self.opcode & 0x0F00) >> 8) {
                            self.registers[i as usize] =
                                self.memory[self.index as usize + i as usize] as u8;
                        }

                        self.index += (((self.opcode & 0x0F00) >> 8) + 1) as usize;
                        self.pc += 2;
                    }

                    x => panic!("unexpected instr in timer: {}", x),
                }
            }

            x => panic!("unexpected instr: {}", x),
        }
        // Update timers
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }
}
