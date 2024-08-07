use bitfield_struct::bitfield;

use crate::cartridge::Cartridge;

#[bitfield(u8)]
struct PPUCTRL {
    /// The first field occupies the least significant bits
    #[bits(2)]
    base_nametable_address: u8,
    /// Booleans are 1 bit large
    vram_address_increment_32: bool,
    upper_sprite_pattern_table: bool,
    upper_background_pattern_table: bool,

    tall_sprites: bool,
    ppu_master: bool,
    gen_nmi_at_vblank: bool,
}

#[bitfield(u8)]
struct PPUMASK {
    greyscale: bool,
    show_background_left: bool,
    show_sprites_left: bool,
    show_background: bool,
    show_sprites: bool,
    emphasize_red: bool,
    emphasize_green: bool,
    emphasize_blue: bool,
}

#[bitfield(u8)]
struct PPUSTATUS {
    #[bits(5)]
    ppu_open_bus: u8,
    sprite_overflow: bool,
    sprite_0_hit: bool,
    vertical_blank_started: bool,
}

#[bitfield(u16)]
struct NametableAddress {
    #[bits(3)]
    fine_y_offset: u8,
    hi_bit_plane: bool,
    #[bits(8)]
    tile_index: u8,
    upper_patter_table: bool,

    #[bits(3)]
    _padding: u8,
}

#[bitfield(u16)]
struct VRAMAddress {
    #[bits(5)]
    coarse_x_scroll: u8,
    #[bits(5)]
    coarse_y_scroll: u8,
    upper_horizontal_nametable: bool,
    upper_vertical_nametable: bool,
    #[bits(3)]
    fine_y_scroll: u8,

    _padding: bool,
}

#[derive(Clone, Copy)]
struct OAMEntry {
    pub y: u8,
    pub tile_index: u8,
    pub attributes: u8,
    pub x: u8,
}

const PALETTE_COLORS: [u8; 192] = [
    0x52, 0x52, 0x52, 0x01, 0x1A, 0x51, 0x0F, 0x0F, 0x65, 0x23, 0x06, 0x63, 0x36, 0x03, 0x4B, 0x40,
    0x04, 0x26, 0x3F, 0x09, 0x04, 0x32, 0x13, 0x00, 0x1F, 0x20, 0x00, 0x0B, 0x2A, 0x00, 0x00, 0x2F,
    0x00, 0x00, 0x2E, 0x0A, 0x00, 0x26, 0x2D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xA0, 0xA0, 0xA0, 0x1E, 0x4A, 0x9D, 0x38, 0x37, 0xBC, 0x58, 0x28, 0xB8, 0x75, 0x21, 0x94, 0x84,
    0x23, 0x5C, 0x82, 0x2E, 0x24, 0x6F, 0x3F, 0x00, 0x51, 0x52, 0x00, 0x31, 0x63, 0x00, 0x1A, 0x6B,
    0x05, 0x0E, 0x69, 0x2E, 0x10, 0x5C, 0x68, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xFE, 0xFF, 0xFF, 0x69, 0x9E, 0xFC, 0x89, 0x87, 0xFF, 0xAE, 0x76, 0xFF, 0xCE, 0x6D, 0xF1, 0xE0,
    0x70, 0xB2, 0xDE, 0x7C, 0x70, 0xC8, 0x91, 0x3E, 0xA6, 0xA7, 0x25, 0x81, 0xBA, 0x28, 0x63, 0xC4,
    0x46, 0x54, 0xC1, 0x7D, 0x56, 0xB3, 0xC0, 0x3C, 0x3C, 0x3C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xFE, 0xFF, 0xFF, 0xBE, 0xD6, 0xFD, 0xCC, 0xCC, 0xFF, 0xDD, 0xC4, 0xFF, 0xEA, 0xC0, 0xF9, 0xF2,
    0xC1, 0xDF, 0xF1, 0xC7, 0xC2, 0xE8, 0xD0, 0xAA, 0xD9, 0xDA, 0x9D, 0xC9, 0xE2, 0x9E, 0xBC, 0xE6,
    0xAE, 0xB4, 0xE5, 0xC7, 0xB5, 0xDF, 0xE4, 0xA9, 0xA9, 0xA9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[allow(clippy::upper_case_acronyms)]
pub struct PPU {
    oam_entries: [OAMEntry; 64],
    oam_addr: u8,
    addr_latch: bool,
    palette: [u8; 32],
    ciram: [u8; 2048],

    status: PPUSTATUS,
    ctrl: PPUCTRL,
    mask: PPUMASK,
    t: VRAMAddress,
    v: VRAMAddress,
    fine_x_scroll: u8,
    ppudata_buffer: u8,

    next_tile: u8,
    next_pattern_lsb: u8,
    next_pattern_msb: u8,
    pattern_plane_0: u16,
    pattern_plane_1: u16,
    sprite_lsb: [u8; 8],
    sprite_msb: [u8; 8],
    num_sprites_on_row: usize,
    temp_oam: [OAMEntry; 8],

    next_attribute: u8,
    attrib_0: u16,
    attrib_1: u16,

    nametable_address: NametableAddress,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            oam_entries: [OAMEntry {
                x: 0,
                y: 0,
                attributes: 0,
                tile_index: 0,
            }; 64],
            oam_addr: 0,
            addr_latch: false,
            palette: [0; 32],
            ciram: [0; 2048],
            fine_x_scroll: 0,
            status: PPUSTATUS(0),
            ctrl: PPUCTRL(0),
            mask: PPUMASK(0),
            t: VRAMAddress(0),
            v: VRAMAddress(0),
            ppudata_buffer: 0,
            next_tile: 0,
            next_pattern_lsb: 0,
            next_pattern_msb: 0,
            pattern_plane_0: 0,
            pattern_plane_1: 0,
            sprite_lsb: [0; 8],
            sprite_msb: [0; 8],
            num_sprites_on_row: 0,
            temp_oam: [OAMEntry {
                attributes: 0,
                tile_index: 0,
                x: 0,
                y: 0,
            }; 8],
            next_attribute: 0,
            attrib_0: 0,
            attrib_1: 0,
            nametable_address: NametableAddress(0),
        }
    }

    pub fn cpu_ppu_bus_read(&mut self, address: u8, cart: &dyn Cartridge) -> u8 {
        let mut value: u8 = 0;

        match address {
            2 => {
                value = self.status.0;
                self.status.set_vertical_blank_started(false);
                self.addr_latch = false;
            }
            4 => {
                let index = self.oam_addr >> 2;
                let which = self.oam_addr & 0b11;
                let entry: &mut OAMEntry = &mut self.oam_entries[index as usize];
                value = match which {
                    0 => entry.y,
                    1 => entry.tile_index,
                    2 => entry.attributes,
                    3 => entry.x,
                    _ => panic!("Should not happen!"),
                }
            }
            7 => {
                value = self.ppudata_buffer;
                self.ppudata_buffer = self.internal_bus_read(self.v.0, cart);

                if self.v.0 >= 0x3f00 && self.v.0 <= 0x3fff {
                    value = self.ppudata_buffer; // Do not delay palette reads
                }

                self.v.0 += if self.ctrl.vram_address_increment_32() {
                    32
                } else {
                    1
                };
            }
            _ => {}
        };

        value
    }

    fn internal_bus_write(&mut self, address: u16, value: u8, cart: &mut dyn Cartridge) {
        if address >= 0x3F00 && address <= 0x3FFF {
            // Palette control
            let index = address & 0xF;
            self.palette[if index == 0 {
                0
            } else {
                (address & 0x1F) as usize
            }] = value;
        } else {
            cart.ppu_write(address, value, &mut self.ciram);
        }
    }

    fn internal_bus_read(&mut self, address: u16, cart: &dyn Cartridge) -> u8 {
        if address >= 0x3F00 && address <= 0x3FFF {
            // Palette control
            let index = address & 0x3;
            return self.palette[if index == 0 {
                0
            } else {
                (address & 0x1F) as usize
            }];
        } else {
            cart.ppu_read(address, &self.ciram)
        }
    }

    pub fn cpu_ppu_bus_write(&mut self, address: u8, value: u8, cart: &mut dyn Cartridge) {
        match address {
            0 => {
                self.ctrl.0 = value;
                self.t.set_upper_horizontal_nametable((value & 1) != 0);
                self.t.set_upper_vertical_nametable(((value >> 1) & 1) != 0);
            }
            1 => {
                self.mask.0 = value;
            }
            3 => {
                self.oam_addr = value;
            }
            4 => {
                let index = self.oam_addr >> 2;
                let which = self.oam_addr & 0b11;
                let entry: &mut OAMEntry = &mut self.oam_entries[index as usize];
                match which {
                    0 => entry.y = value,
                    1 => entry.tile_index = value,
                    2 => entry.attributes = value,
                    3 => entry.x = value,
                    _ => panic!("Should not happen!"),
                };
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            5 => {
                if self.addr_latch {
                    self.t.set_coarse_y_scroll((value >> 3) & 0b11111);
                    self.t.set_fine_y_scroll(value & 0b111);
                } else {
                    self.t.set_coarse_x_scroll((value >> 3) & 0b11111);
                    self.fine_x_scroll = value & 0b111;
                }
                self.addr_latch = !self.addr_latch;
            }
            6 => {
                if self.addr_latch {
                    self.t.0 = (self.t.0 & 0xFF00) | (value as u16);
                    self.v.0 = self.t.0;
                } else {
                    let temp = ((value & 0x7F) as u16) << 8;
                    self.t.0 = temp | (self.t.0 & 0xFF);
                    self.t.0 &= 0x7FFF; // Clear top bit
                }
                self.addr_latch = !self.addr_latch;
            }
            7 => {
                self.internal_bus_write(self.v.0, value, cart);
                self.v.0 += if self.ctrl.vram_address_increment_32() {
                    32
                } else {
                    1
                };
            }
            _ => {}
            // _ => panic!("Out of range"),
        };
    }

    #[inline]
    fn nametable_fetch(&mut self, cart: &dyn Cartridge) {
        self.next_tile = self.internal_bus_read(0x2000 | (self.v.0 & 0x0FFF), cart);
    }

    #[inline]
    fn attribute_fetch(&mut self, cart: &dyn Cartridge) {
        self.next_attribute = self.internal_bus_read(
            0x23C0 | (self.v.0 & 0x0C00) | ((self.v.0 >> 4) & 0x38) | ((self.v.0 >> 2) & 0x07),
            cart,
        );
        if (self.v.coarse_y_scroll() & 2) != 0 {
            self.next_attribute >>= 4;
        }
        if (self.v.coarse_x_scroll() & 2) != 0 {
            self.next_attribute >>= 2;
        }
        self.next_attribute &= 0b11;
    }

    #[inline]
    fn bg_lsb_fetch(&mut self, cart: &dyn Cartridge) {
        self.nametable_address
            .set_fine_y_offset(self.v.fine_y_scroll());
        self.nametable_address.set_hi_bit_plane(false);
        self.nametable_address.set_tile_index(self.next_tile);
        self.nametable_address
            .set_upper_patter_table(self.ctrl.upper_background_pattern_table());
        self.next_pattern_lsb = self.internal_bus_read(self.nametable_address.0, cart);
    }

    #[inline]
    fn bg_msb_fetch(&mut self, cart: &dyn Cartridge) {
        self.nametable_address.set_hi_bit_plane(true);
        self.next_pattern_msb = self.internal_bus_read(self.nametable_address.0, cart);
    }

    #[inline]
    fn inc_horiz(&mut self) {
        if !self.mask.show_background() {
            return;
        }

        if self.v.coarse_x_scroll() == 31 {
            self.v.set_coarse_x_scroll(0);
            self.v
                .set_upper_horizontal_nametable(!self.v.upper_horizontal_nametable());
        } else {
            self.v
                .set_coarse_x_scroll(self.v.coarse_x_scroll().wrapping_add(1));
        }
    }

    #[inline]
    fn inc_vert(&mut self) {
        if !self.mask.show_background() {
            return;
        }

        if self.v.fine_y_scroll() < 7 {
            self.v
                .set_fine_y_scroll(self.v.fine_y_scroll().wrapping_add(1));
        } else {
            self.v.set_fine_y_scroll(0);
            if self.v.coarse_y_scroll() == 29 {
                self.v.set_coarse_y_scroll(0);
                self.v
                    .set_upper_vertical_nametable(!self.v.upper_vertical_nametable());
            } else if self.v.coarse_y_scroll() == 31 {
                self.v.set_coarse_y_scroll(0);
            } else {
                self.v
                    .set_coarse_y_scroll(self.v.coarse_y_scroll().wrapping_add(1));
            }
        }
    }

    #[inline]
    fn load_shifters(&mut self) {
        self.pattern_plane_0 |= self.next_pattern_lsb as u16;
        self.pattern_plane_1 |= self.next_pattern_msb as u16;

        self.attrib_0 |= if (self.next_attribute & 1) != 0 {
            0xFF
        } else {
            0
        };

        self.attrib_1 |= if (self.next_attribute & 2) != 0 {
            0xFF
        } else {
            0
        };
    }

    pub fn tick(&mut self, scanline: i32, dot: u16, fb: &mut [u32], cart: &dyn Cartridge) -> bool {
        if scanline <= 239 {
            if scanline == -1 && dot == 1 {
                self.status.set_vertical_blank_started(false);
                self.status.set_sprite_overflow(false);
                self.status.set_sprite_0_hit(false);
            }

            if (dot >= 2 && dot < 258) || (dot >= 321 && dot < 338) {
                if self.mask.show_background() {
                    self.pattern_plane_0 <<= 1;
                    self.pattern_plane_1 <<= 1;
                    self.attrib_0 <<= 1;
                    self.attrib_1 <<= 1;
                }

                match (dot - 1) % 8 {
                    0 => {
                        self.load_shifters();
                        self.nametable_fetch(cart);
                    }
                    2 => {
                        self.attribute_fetch(cart);
                    }
                    4 => {
                        self.bg_lsb_fetch(cart);
                    }
                    6 => {
                        self.bg_msb_fetch(cart);
                    }
                    7 => self.inc_horiz(),
                    _ => {}
                };
            }

            if dot == 256 {
                self.inc_vert();
            } else if dot == 257 {
                self.load_shifters();

                if self.mask.show_background() || self.mask.show_sprites() {
                    self.v
                        .set_upper_horizontal_nametable(self.t.upper_horizontal_nametable());
                    self.v.set_coarse_x_scroll(self.t.coarse_x_scroll());
                }

                if self.mask.show_sprites() {
                    for i in 0..8 {
                        self.temp_oam[i].x = 0xFF;
                        self.temp_oam[i].y = 0xFF;
                        self.temp_oam[i].attributes = 0xFF;
                        self.temp_oam[i].tile_index = 0xFF;
                    }

                    self.num_sprites_on_row = 0;

                    for i in 0..64 {
                        let delta_y = scanline - (self.oam_entries[i].y as i32);
                        if delta_y >= 0
                            && (if self.ctrl.tall_sprites() {
                                delta_y < 16
                            } else {
                                delta_y < 8
                            })
                            && self.num_sprites_on_row < 8
                        {
                            self.temp_oam[self.num_sprites_on_row].y = self.oam_entries[i].y;
                            self.temp_oam[self.num_sprites_on_row].tile_index =
                                self.oam_entries[i].tile_index;
                            self.temp_oam[self.num_sprites_on_row].attributes =
                                self.oam_entries[i].attributes;
                            self.temp_oam[self.num_sprites_on_row].x = self.oam_entries[i].x;
                            self.num_sprites_on_row += 1;
                            if self.num_sprites_on_row == 8 {
                                self.status.set_sprite_overflow(true);
                            }
                        }
                    }
                }
            } else if dot == 338 {
                self.nametable_fetch(cart);
            } else if dot == 340 {
                self.nametable_fetch(cart);

                if self.mask.show_sprites() {
                    for i in 0..self.num_sprites_on_row {
                        if self.ctrl.tall_sprites() {
                            let flipped_y = (self.temp_oam[i].attributes & 0x80) != 0;
                            let mut y_offset = scanline - (self.temp_oam[i].y as i32);

                            if flipped_y {
                                y_offset = 15 - y_offset;
                            }

                            let mut sprite_index = self.temp_oam[i].tile_index & 0xFE;
                            if y_offset > 7 {
                                y_offset -= 8;
                                sprite_index += 1;
                            }

                            self.nametable_address.set_fine_y_offset(y_offset as u8);
                            self.nametable_address.set_hi_bit_plane(false);
                            self.nametable_address.set_tile_index(sprite_index);
                            self.nametable_address
                                .set_upper_patter_table((self.temp_oam[i].tile_index & 1) == 1);
                        } else {
                            self.nametable_address
                                .set_fine_y_offset((scanline - (self.temp_oam[i].y as i32)) as u8);
                            let flipped_y = (self.temp_oam[i].attributes & 0x80) != 0;
                            if flipped_y {
                                self.nametable_address
                                    .set_fine_y_offset(7 - self.nametable_address.fine_y_offset());
                            }

                            self.nametable_address.set_hi_bit_plane(false);
                            self.nametable_address
                                .set_tile_index(self.temp_oam[i].tile_index);
                            self.nametable_address
                                .set_upper_patter_table(self.ctrl.upper_sprite_pattern_table());
                        }

                        self.sprite_lsb[i] = cart.ppu_read(self.nametable_address.0, &self.ciram);
                        self.nametable_address.set_hi_bit_plane(true);
                        self.sprite_msb[i] = cart.ppu_read(self.nametable_address.0, &self.ciram);
                    }
                }
            }

            if scanline == -1 && dot >= 280 && dot <= 304 && self.mask.show_background() {
                self.v.set_coarse_y_scroll(self.t.coarse_y_scroll());
                self.v.set_fine_y_scroll(self.t.fine_y_scroll());
                self.v
                    .set_upper_vertical_nametable(self.t.upper_vertical_nametable());
            }

            if scanline >= 0 && dot >= 1 && dot <= 256 {
                let mut bg_pixel: u8 = 0;
                let mut bg_palette: u8 = 0;

                if self.mask.show_background() {
                    let bit: u16 = 0x8000 >> self.fine_x_scroll;

                    let lo_bit = if (self.pattern_plane_0 & bit) != 0 {
                        1
                    } else {
                        0
                    };
                    let hi_bit = if (self.pattern_plane_1 & bit) != 0 {
                        1
                    } else {
                        0
                    };
                    bg_pixel = (hi_bit << 1) | lo_bit;

                    let attr_lo = if (self.attrib_0 & bit) != 0 { 1 } else { 0 };
                    let attr_hi = if (self.attrib_1 & bit) != 0 { 1 } else { 0 };
                    bg_palette = (attr_hi << 3) | (attr_lo << 2);
                }

                let mut first_found: i32 = -1;
                let mut sprite_pixel: u8 = 0;
                let mut sprite_palette: u8 = 0;

                if self.mask.show_sprites() {
                    for sprite_n in 0..self.num_sprites_on_row {
                        if self.temp_oam[sprite_n].x == 0 {
                            let flipped_x = (self.temp_oam[sprite_n].attributes & 0x40) != 0;
                            if sprite_pixel == 0 {
                                #[rustfmt::skip]
                                let lo_bit = if (self.sprite_lsb[sprite_n] & (if flipped_x { 1} else {0x80})) != 0 { 1 } else { 0 };
                                #[rustfmt::skip]
                                let hi_bit = if (self.sprite_msb[sprite_n] & (if flipped_x { 1} else {0x80})) != 0 { 1 } else { 0 };
                                let pix = (hi_bit << 1) | lo_bit;

                                if pix != 0 {
                                    first_found = sprite_n as i32;
                                    if sprite_n == 0
                                        && bg_pixel != 0
                                        && self.temp_oam[0].y == self.oam_entries[0].y
                                    {
                                        self.status.set_sprite_0_hit(true);
                                    }

                                    sprite_pixel = pix;
                                    sprite_palette =
                                        (self.temp_oam[sprite_n].attributes & 0b11) << 2;
                                }
                            }

                            if flipped_x {
                                self.sprite_lsb[sprite_n] >>= 1;
                                self.sprite_msb[sprite_n] >>= 1;
                            } else {
                                self.sprite_lsb[sprite_n] <<= 1;
                                self.sprite_msb[sprite_n] <<= 1;
                            }
                        } else {
                            self.temp_oam[sprite_n].x -= 1;
                        }
                    }
                }

                let mut output_palette_location: u16 = 0x00;
                let mut output_pixel = bg_pixel;
                let mut output_palette = bg_palette;

                if self.mask.show_sprites() {
                    if bg_pixel == 0 && sprite_pixel != 0 {
                        output_pixel = sprite_pixel;
                        output_palette = sprite_palette;
                        output_palette_location = 0x10;
                    } else if sprite_pixel != 0 && bg_pixel != 0 {
                        if first_found >= 0
                            && (((self.temp_oam[first_found as usize].attributes >> 5) & 1) == 0)
                        {
                            output_pixel = sprite_pixel;
                            output_palette = sprite_palette;
                            output_palette_location = 0x10;
                        }
                    }
                }

                let palette_addr: u16 = output_palette_location | (output_palette as u16) | (output_pixel as u16);
                let palette_index = (self.palette[if (palette_addr & 0x3) == 0 { 0 } else { (palette_addr & 0x1F) as usize}] & 0x3f) as usize;
                
                let pixel = &mut fb[256 * (scanline as usize) + ((dot as usize) - 1)];
                *pixel = (0xFF << 24)
                    | ((PALETTE_COLORS[palette_index * 3] as u32) << 16)
                    | ((PALETTE_COLORS[(palette_index * 3) + 1] as u32) << 8)
                    | (PALETTE_COLORS[(palette_index * 3) + 2] as u32);
            }
        }

        if scanline == 241 && dot == 1 {
            self.status.set_vertical_blank_started(true);
            if self.ctrl.gen_nmi_at_vblank() {
                return true;
            }
        }

        false
    }
}
