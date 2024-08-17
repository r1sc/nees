#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod reader_writer;

mod apu;
mod bit_helpers;
mod bus;
mod cartridge;
mod cpu;
mod ines;
mod mappers;
mod ppu;

pub mod nes001;
pub use reader_writer::{EasyReader, EasyWriter};