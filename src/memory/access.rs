/// A value associated with a read/write
#[derive(Debug, Clone, Copy)]
pub enum MemAccessVal {
    U8(u8),
    U16(u16),
    U32(u32),
}

/// Memory Access Kind (Read or Write)
#[derive(Debug, Copy, Clone)]
pub enum MemAccessKind {
    Read,
    Write,
}

/// Encodes a memory access (read/write)
#[derive(Debug, Clone, Copy)]
pub struct MemAccess {
    pub kind: MemAccessKind,
    pub offset: u32,
    pub val: MemAccessVal,
}

macro_rules! impl_memaccess {
    ($kind:ident, $fn:ident, $val:ty, $size:ident) => {
        pub fn $fn(offset: u32, val: $val) -> MemAccess {
            MemAccess {
                kind: MemAccessKind::$kind,
                offset,
                val: MemAccessVal::$size(val),
            }
        }
    };
}

impl MemAccess {
    impl_memaccess!(Read, r8, u8, U8);
    impl_memaccess!(Read, r16, u16, U16);
    impl_memaccess!(Read, r32, u32, U32);
    impl_memaccess!(Write, w8, u8, U8);
    impl_memaccess!(Write, w16, u16, U16);
    impl_memaccess!(Write, w32, u32, U32);
}

impl std::fmt::Display for MemAccessVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemAccessVal::U8(val) => write!(f, "{:#04x?}", val),
            MemAccessVal::U16(val) => write!(f, "{:#06x?}", val),
            MemAccessVal::U32(val) => write!(f, "{:#010x?}", val),
        }
    }
}

impl std::fmt::Display for MemAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            MemAccessKind::Read => write!(
                f,
                "{}({:#010x?}) // {}",
                match self.val {
                    MemAccessVal::U8(_) => "r8",
                    MemAccessVal::U16(_) => "r16",
                    MemAccessVal::U32(_) => "r32",
                },
                self.offset,
                self.val
            ),
            MemAccessKind::Write => write!(
                f,
                "{}({:#010x?}, {})",
                match self.val {
                    MemAccessVal::U8(_) => "w8",
                    MemAccessVal::U16(_) => "w16",
                    MemAccessVal::U32(_) => "w32",
                },
                self.offset,
                self.val
            ),
        }
    }
}
