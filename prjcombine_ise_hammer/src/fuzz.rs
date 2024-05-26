use prjcombine_hammer::Session;
use prjcombine_int::db::{BelId, NodeKindId};

use crate::{backend::IseBackend, fgen::TileBits};

pub struct FuzzCtx<'sm, 's, 'a> {
    pub session: &'sm mut Session<'s, IseBackend<'a>>,
    pub node_kind: NodeKindId,
    pub bits: TileBits,
    pub tile_name: &'a str,
    pub bel: BelId,
    pub bel_name: &'a str,
}

#[macro_export]
macro_rules! fuzz_wire {
    ($ctx:ident, (pin $pin:expr)) => {
        $crate::fgen::TileWire::BelPinNear($ctx.bel, $pin)
    };
    ($ctx:ident, (bel_pin $bel:expr, $pin:expr)) => {
        $crate::fgen::TileWire::BelPinNear($bel, $pin)
    };
}

#[macro_export]
macro_rules! fuzz_base {
    ($ctx:ident, (mode $kind:expr)) => {
        $crate::fgen::TileKV::SiteMode($ctx.bel, $kind)
    };
    ($ctx:ident, (bel_mode $bel:expr, $kind:expr)) => {
        $crate::fgen::TileKV::SiteMode($bel, $kind)
    };
    ($ctx:ident, (bel_unused $bel:expr)) => {
        $crate::fgen::TileKV::SiteUnused($bel)
    };
    ($ctx:ident, (attr $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::SiteAttr($ctx.bel, $attr, $val)
    };
    ($ctx:ident, (bel_attr $bel:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::SiteAttr($bel, $attr, $val)
    };
    ($ctx:ident, (pin $pin:expr)) => {
        $crate::fgen::TileKV::SitePin($ctx.bel, $pin)
    };
    ($ctx:ident, (mutex $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::SiteMutex($ctx.bel, $attr, $val)
    };
    ($ctx:ident, (global_mutex_none $name:expr)) => {
        $crate::fgen::TileKV::GlobalMutexNone($name)
    };
    ($ctx:ident, (global_mutex_site $name:expr)) => {
        $crate::fgen::TileKV::GlobalMutexSite($name, $ctx.bel)
    };
    ($ctx:ident, (row_mutex_site $name:expr)) => {
        $crate::fgen::TileKV::RowMutexSite($name, $ctx.bel)
    };
    ($ctx:ident, (pip $wa:tt, $wb:tt)) => {
        $crate::fgen::TileKV::Pip($crate::fuzz_wire!($ctx, $wa), $crate::fuzz_wire!($ctx, $wb))
    };
}

#[macro_export]
macro_rules! fuzz_diff {
    ($ctx:ident, (mode $kind:expr)) => {
        $crate::fgen::TileFuzzKV::SiteMode($ctx.bel, $kind)
    };
    ($ctx:ident, (attr $attr:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::SiteAttr($ctx.bel, $attr, $val)
    };
    ($ctx:ident, (global_opt $opt:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::GlobalOpt($opt, $val)
    };
    ($ctx:ident, (global_opt_diff $opt:expr, $vala:expr, $valb:expr)) => {
        $crate::fgen::TileFuzzKV::GlobalOptDiff($opt, $vala, $valb)
    };
    ($ctx:ident, (pip $wa:tt, $wb:tt)) => {
        $crate::fgen::TileFuzzKV::Pip($crate::fuzz_wire!($ctx, $wa), $crate::fuzz_wire!($ctx, $wb))
    };
}

#[macro_export]
macro_rules! fuzz_diff_multi {
    ($ctx:ident, (attr_lut $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr($ctx.bel, $attr, $crate::backend::MultiValue::Lut)
    };
    ($ctx:ident, (attr_hex $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr,
            $crate::backend::MultiValue::Hex(0),
        )
    };
    ($ctx:ident, (attr_hex_delta $attr:expr, $delta:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr,
            $crate::backend::MultiValue::Hex($delta),
        )
    };
}

#[macro_export]
macro_rules! fuzz_one {
    ($ctx:ident, $attr:expr, $val:expr, [$($base:tt),*], [$($diff:tt),*]) => {
        $ctx.session.add_fuzzer(Box::new($crate::fgen::TileFuzzerGen {
            node: $ctx.node_kind,
            bits: $ctx.bits,
            feature: $crate::backend::SimpleFeatureId {
                tile: $ctx.tile_name,
                bel: $ctx.bel_name,
                attr: $attr,
                val: $val,
            },
            base: vec![
                $($crate::fuzz_base!($ctx, $base)),*
            ],
            fuzz: vec![
                $($crate::fuzz_diff!($ctx, $diff)),*
            ]
        }));
    };
}

#[macro_export]
macro_rules! fuzz_multi {
    ($ctx:ident, $attr:expr, $val:expr, $width:expr, [$($base:tt),*], $diff:tt) => {
        $ctx.session.add_fuzzer(Box::new($crate::fgen::TileMultiFuzzerGen {
            node: $ctx.node_kind,
            bits: $ctx.bits,
            feature: $crate::backend::SimpleFeatureId {
                tile: $ctx.tile_name,
                bel: $ctx.bel_name,
                attr: $attr,
                val: $val,
            },
            base: vec![
                $($crate::fuzz_base!($ctx, $base)),*
            ],
            width: $width,
            fuzz: $crate::fuzz_diff_multi!($ctx, $diff),
        }));
    };
}

#[macro_export]
macro_rules! fuzz_enum {
    ($ctx:ident, $attr:expr, $vals:expr, [$($base:tt),*]) => {
        for val in $vals {
            $crate::fuzz_one!($ctx, $attr, val, [$($base),*], [(attr $attr, val)]);
        }
    }
}
