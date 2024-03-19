use enumflags2::bitflags;
pub use enumflags2::BitFlags;

#[derive(Clone, Copy, Debug)]
#[bitflags]
#[repr(u8)]
pub enum Flag {
    CliMode = 1,
    FirstInit = 1 << 1,
    ErrorDuringInit = 1 << 2,
    PortableMode = 1 << 3,
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_flags() {
        let mut flags = BitFlags::EMPTY;
        println!("{:?}", flags.exactly_one());
        flags.insert(Flag::FirstInit);
        println!("{flags:?}");
        assert!(flags.contains(Flag::FirstInit));
        println!("{:?}", flags.exactly_one())
    }
}
