use crate::{cartridge::Cartridge, ines::INES};

mod mmc1;
mod nrom;
mod unrom;
mod mmc2;
mod mmc3;

pub fn load_cart(ines: INES) -> Box<dyn Cartridge + Sync + Send> {
    match ines.mapper_no {
        0 => Box::new(nrom::NROM::new(ines)),
        1 => Box::new(mmc1::MMC1::new(ines)),
        2 => Box::new(unrom::UNROM::new(ines)),
        4 => Box::new(mmc3::MMC3::new(ines)),
        9 => Box::new(mmc2::MMC2::new(ines)),
        _ => panic!("Unsupported mapper {}", ines.mapper_no),
    }
}