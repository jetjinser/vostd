#![no_std]

/// Runtime-only power-of-two alignment helpers needed by the OSDK image builder.
pub trait AlignExt: Sized {
    fn align_up(self, align: Self) -> Self;
    fn align_down(self, align: Self) -> Self;
}

macro_rules! impl_align_ext {
    ($($ty:ty),+ $(,)?) => {$ (
        impl AlignExt for $ty {
            fn align_up(self, align: Self) -> Self {
                assert!(align >= 2 && align.is_power_of_two());
                self.checked_add(align - 1).expect("alignment overflow") & !(align - 1)
            }

            fn align_down(self, align: Self) -> Self {
                assert!(align >= 2 && align.is_power_of_two());
                self & !(align - 1)
            }
        }
    )+};
}

impl_align_ext!(u8, u16, u32, u64, usize);
