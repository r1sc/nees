pub trait EasyWriter {
    fn write_u8(&mut self, value: u8) -> anyhow::Result<()>;
    fn write_u16(&mut self, value: u16) -> anyhow::Result<()>;
    fn write_i16(&mut self, value: i16) -> anyhow::Result<()>;
    fn write_u32(&mut self, value: u32) -> anyhow::Result<()>;
    fn write_bool(&mut self, value: bool) -> anyhow::Result<()>;
    fn write_all(&mut self, buf: &[u8]) -> anyhow::Result<()>;
}

pub trait EasyReader {
    fn read_u8(&mut self) -> anyhow::Result<u8>;
    fn read_u16(&mut self) -> anyhow::Result<u16>;
    fn read_i16(&mut self) -> anyhow::Result<i16>;
    fn read_u32(&mut self) -> anyhow::Result<u32>;
    fn read_bool(&mut self) -> anyhow::Result<bool>;
    fn read_exact(&mut self, buf: &mut [u8]) -> anyhow::Result<()>;
}