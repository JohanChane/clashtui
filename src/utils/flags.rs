use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct Flag: u8 {
        const FirstInit = 1;
        const PortableMode = 1<<1;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_flags() {
        let mut flags = Flag::empty();
        flags.insert(Flag::FirstInit);
        println!("{flags:?}");
        assert!(flags.contains(Flag::FirstInit));
        flags.insert(Flag::PortableMode);
        println!("{flags:?}");
        assert!(flags.contains(Flag::FirstInit));
        assert!(flags.contains(Flag::PortableMode));
    }
}
