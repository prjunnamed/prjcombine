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
macro_rules! fuzz_base {
    ($ctx:ident, (mode $kind:expr)) => {
        TileKV::SiteMode($ctx.bel, $kind)
    };
    ($ctx:ident, (attr $attr:expr, $val:expr)) => {
        TileKV::SiteAttr($ctx.bel, $attr, $val)
    };
    ($ctx:ident, (pin $pin:expr)) => {
        TileKV::SitePin($ctx.bel, $pin)
    };
}

#[macro_export]
macro_rules! fuzz_diff {
    ($ctx:ident, (attr $attr:expr, $val:expr)) => {
        TileFuzzKV::SiteAttr($ctx.bel, $attr, $val)
    };
}

#[macro_export]
macro_rules! fuzz_diff_multi {
    ($ctx:ident, (attr_lut $attr:expr)) => {
        TileMultiFuzzKV::SiteAttr($ctx.bel, $attr, MultiValue::Lut)
    };
}

#[macro_export]
macro_rules! fuzz_one {
    ($ctx:ident, $attr:expr, $val:expr, [$($base:tt),*], [$($diff:tt),*]) => {
        $ctx.session.add_fuzzer(Box::new(TileFuzzerGen {
            node: $ctx.node_kind,
            bits: $ctx.bits,
            feature: SimpleFeatureId {
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
        $ctx.session.add_fuzzer(Box::new(TileMultiFuzzerGen {
            node: $ctx.node_kind,
            bits: $ctx.bits,
            feature: SimpleFeatureId {
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
