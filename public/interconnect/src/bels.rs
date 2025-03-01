#[macro_export]
macro_rules! bels {
    (@CONSTS@ ($idx:expr) $slot:ident, $($rest:ident,)*) => {
        pub const $slot: $crate::db::BelSlotId = $crate::db::BelSlotId::from_idx_const($idx);
        $crate::bels!(@CONSTS@ ($slot.to_idx_const() + 1) $($rest,)*);
    };
    (@CONSTS@ ($idx:expr)) => {};
    ($($slot:ident),* $(,)?) => {
        $crate::bels!(@CONSTS@ (0) $($slot,)*);
        pub const SLOTS: &[&str] = &[
            $( stringify!($slot) ),*
        ];
    };
}
