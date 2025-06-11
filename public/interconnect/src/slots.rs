#[macro_export]
macro_rules! bels {
    (@CONSTS@ ($idx:expr) $slot:ident, $($rest:ident,)*) => {
        pub const $slot: $crate::db::BelSlotId = $crate::db::BelSlotId::from_idx_const($idx);
        $crate::bels!(@CONSTS@ ($slot.to_idx_const() + 1) $($rest,)*);
    };
    (@CONSTS@ ($idx:expr)) => {};
    ($($slot:ident : $tslot:expr),* $(,)?) => {
        $crate::bels!(@CONSTS@ (0) $($slot,)*);
        pub const SLOTS: &[($crate::db::BelSlotId, &str, $crate::db::TileSlotId)] = &[
            $( ($slot, stringify!($slot), $tslot) ),*
        ];
    };
}

#[macro_export]
macro_rules! tile_slots {
    (@CONSTS@ ($idx:expr) $slot:ident, $($rest:ident,)*) => {
        pub const $slot: $crate::db::TileSlotId = $crate::db::TileSlotId::from_idx_const($idx);
        $crate::tile_slots!(@CONSTS@ ($slot.to_idx_const() + 1) $($rest,)*);
    };
    (@CONSTS@ ($idx:expr)) => {};
    ($($slot:ident),* $(,)?) => {
        $crate::tile_slots!(@CONSTS@ (0) $($slot,)*);
        pub const SLOTS: &[($crate::db::TileSlotId, &str)] = &[
            $( ($slot, stringify!($slot)) ),*
        ];
    };
}
