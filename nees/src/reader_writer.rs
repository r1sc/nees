use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub trait EasyWriter {
    fn write_u8(&mut self, value: u8) -> std::io::Result<()>;
    fn write_u16(&mut self, value: u16) -> std::io::Result<()>;
    fn write_u32(&mut self, value: u32) -> std::io::Result<()>;
    fn write_bool(&mut self, value: bool) -> std::io::Result<()>;
}

impl<T> EasyWriter for T
where
    T: std::io::Write,
{
    fn write_u8(&mut self, value: u8) -> std::io::Result<()> {
        WriteBytesExt::write_u8(self, value)
    }

    fn write_u16(&mut self, value: u16) -> std::io::Result<()> {
        WriteBytesExt::write_u16::<LittleEndian>(self, value)
    }

    fn write_u32(&mut self, value: u32) -> std::io::Result<()> {
        WriteBytesExt::write_u32::<LittleEndian>(self, value)
    }

    fn write_bool(&mut self, value: bool) -> std::io::Result<()> {
        WriteBytesExt::write_u8(self, if value { 1 } else { 0 })
    }
}

pub trait EasyReader {
    fn read_u8(&mut self) -> std::io::Result<u8>;
    fn read_u16(&mut self) -> std::io::Result<u16>;
    fn read_u32(&mut self) -> std::io::Result<u32>;
    fn read_bool(&mut self) -> std::io::Result<bool>;
}

impl<T> EasyReader for T
where
    T: std::io::Read,
{
    fn read_u8(&mut self) -> std::io::Result<u8> {
        ReadBytesExt::read_u8(self)
    }

    fn read_u16(&mut self) -> std::io::Result<u16> {
        ReadBytesExt::read_u16::<LittleEndian>(self)
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        ReadBytesExt::read_u32::<LittleEndian>(self)
    }

    fn read_bool(&mut self) -> std::io::Result<bool> {
        Ok(ReadBytesExt::read_u8(self)? == 1)
    }
}