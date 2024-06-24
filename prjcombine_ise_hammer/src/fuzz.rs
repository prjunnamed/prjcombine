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
    ($ctx:ident, (pin_far $pin:expr)) => {
        $crate::fgen::TileWire::BelPinFar($ctx.bel, $pin)
    };
    ($ctx:ident, (bel_pin $bel:expr, $pin:expr)) => {
        $crate::fgen::TileWire::BelPinNear($bel, $pin)
    };
    ($ctx:ident, (bel_pin_far $bel:expr, $pin:expr)) => {
        $crate::fgen::TileWire::BelPinFar($bel, $pin)
    };
    ($ctx:ident, (related_pin $relation:expr, $pin:expr)) => {
        $crate::fgen::TileWire::RelatedBelPinNear($ctx.bel, $relation, $pin)
    };
}

#[macro_export]
macro_rules! fuzz_base {
    ($ctx:ident, (mode $kind:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Mode($kind))
    };
    ($ctx:ident, (bel_mode $bel:expr, $kind:expr)) => {
        $crate::fgen::TileKV::Bel($bel, $crate::fgen::BelKV::Mode($kind))
    };
    ($ctx:ident, (bel_unused $bel:expr)) => {
        $crate::fgen::TileKV::Bel($bel, $crate::fgen::BelKV::Unused)
    };
    ($ctx:ident, (iob_mode $iob:expr, $kind:expr)) => {
        $crate::fgen::TileKV::IobBel($iob.tile, $iob.bel, $crate::fgen::BelKV::Mode($kind))
    };
    ($ctx:ident, (attr $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Attr($attr, $val))
    };
    ($ctx:ident, (bel_attr $bel:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel($bel, $crate::fgen::BelKV::Attr($attr, $val))
    };
    ($ctx:ident, (iob_attr $iob:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::IobBel($iob.tile, $iob.bel, $crate::fgen::BelKV::Attr($attr, $val))
    };
    ($ctx:ident, (pin $pin:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Pin($pin, true))
    };
    ($ctx:ident, (iob_pin $iob:expr, $pin:expr)) => {
        $crate::fgen::TileKV::IobBel($iob.tile, $iob.bel, $crate::fgen::BelKV::Pin($pin, true))
    };
    ($ctx:ident, (nopin $pin:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Pin($pin, false))
    };
    ($ctx:ident, (bel_special $special:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $special)
    };
    ($ctx:ident, (iob_special $iob:expr, $special:expr)) => {
        $crate::fgen::TileKV::IobBel($iob.tile, $iob.bel, $special)
    };
    ($ctx:ident, (mutex $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Mutex($attr, $val))
    };
    ($ctx:ident, (bel_mutex $bel:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel($bel, $crate::fgen::BelKV::Mutex($attr, $val))
    };
    ($ctx:ident, (tile_mutex $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::TileMutex($attr, $val)
    };
    ($ctx:ident, (package $pkg:expr)) => {
        $crate::fgen::TileKV::Package($pkg)
    };
    ($ctx:ident, (vccaux $val:expr)) => {
        $crate::fgen::TileKV::VccAux($val)
    };
    ($ctx:ident, (global_opt $name:expr, $val:expr)) => {
        $crate::fgen::TileKV::GlobalOpt($name, $val)
    };
    ($ctx:ident, (no_global_opt $name:expr)) => {
        $crate::fgen::TileKV::NoGlobalOpt($name)
    };
    ($ctx:ident, (global_mutex_none $name:expr)) => {
        $crate::fgen::TileKV::GlobalMutexNone($name)
    };
    ($ctx:ident, (global_mutex $name:expr, $val:expr)) => {
        $crate::fgen::TileKV::GlobalMutex($name, $val)
    };
    ($ctx:ident, (global_mutex_site $name:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::GlobalMutexHere($name))
    };
    ($ctx:ident, (row_mutex_site $name:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::RowMutexHere($name))
    };
    ($ctx:ident, (pip $wa:tt, $wb:tt)) => {
        $crate::fgen::TileKV::Pip($crate::fuzz_wire!($ctx, $wa), $crate::fuzz_wire!($ctx, $wb))
    };
    ($ctx:ident, (related $rel:expr, $inner:tt)) => {
        $crate::fgen::TileKV::TileRelated($rel, Box::new($crate::fuzz_base!($ctx, $inner)))
    };
    ($ctx:ident, (special $val:expr)) => {
        $val
    };
}

#[macro_export]
macro_rules! fuzz_diff {
    ($ctx:ident, (mode $kind:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $crate::fgen::BelFuzzKV::Mode($kind))
    };
    ($ctx:ident, (attr $attr:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $crate::fgen::BelFuzzKV::Attr($attr, $val))
    };
    ($ctx:ident, (attr_diff $attr:expr, $vala:expr, $valb:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $ctx.bel,
            $crate::fgen::BelFuzzKV::AttrDiff($attr, $vala, $valb),
        )
    };
    ($ctx:ident, (pin $pin:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $crate::fgen::BelFuzzKV::Pin($pin))
    };
    ($ctx:ident, (pin_full $pin:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $crate::fgen::BelFuzzKV::PinFull($pin))
    };
    ($ctx:ident, (iob_mode $iob:expr, $kind:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel($iob.tile, $iob.bel, $crate::fgen::BelFuzzKV::Mode($kind))
    };
    ($ctx:ident, (iob_mode_diff $iob:expr, $kinda:expr, $kindb:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelFuzzKV::ModeDiff($kinda, $kindb),
        )
    };
    ($ctx:ident, (iob_attr $iob:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelFuzzKV::Attr($attr, $val),
        )
    };
    ($ctx:ident, (iob_attr_diff $iob:expr, $attr:expr, $vala:expr, $valb:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelFuzzKV::AttrDiff($attr, $vala, $valb),
        )
    };
    ($ctx:ident, (iob_pin $iob:expr, $pin:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel($iob.tile, $iob.bel, $crate::fgen::BelFuzzKV::Pin($pin))
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
    ($ctx:ident, (row_mutex $name:expr)) => {
        $crate::fgen::TileFuzzKV::RowMutexExclusive($name)
    };
    ($ctx:ident, (related $rel:expr, $inner:tt)) => {
        $crate::fgen::TileFuzzKV::TileRelated($rel, Box::new($crate::fuzz_diff!($ctx, $inner)))
    };
    ($ctx:ident, (special $val:expr)) => {
        $val
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
    ($ctx:ident, (attr_bin $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr($ctx.bel, $attr, $crate::backend::MultiValue::Bin)
    };
    ($ctx:ident, (attr_dec $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr($ctx.bel, $attr, $crate::backend::MultiValue::Dec)
    };
    ($ctx:ident, (iob_attr_bin $iob:expr, $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::IobSiteAttr(
            $iob.tile,
            $iob.bel,
            $attr,
            $crate::backend::MultiValue::Bin,
        )
    };
    ($ctx:ident, (global_hex_prefix $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::GlobalOpt($attr, $crate::backend::MultiValue::HexPrefix)
    };
    ($ctx:ident, (global_bin $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::GlobalOpt($attr, $crate::backend::MultiValue::Bin)
    };
    ($ctx:ident, (global_dec $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::GlobalOpt($attr, $crate::backend::MultiValue::Dec)
    };
}

#[macro_export]
macro_rules! fuzz_one {
    ($ctx:ident, $attr:expr, $val:expr, [$($base:tt),*], [$($diff:tt),*]) => {
        $ctx.session.add_fuzzer(Box::new($crate::fgen::TileFuzzerGen {
            node: $ctx.node_kind,
            bits: $ctx.bits.clone(),
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
            bits: $ctx.bits.clone(),
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
macro_rules! fuzz_multi_attr_dec {
    ($ctx:ident, $attr:expr, $width:expr, [$($base:tt),*]) => {
        $crate::fuzz_multi!($ctx, $attr, "", $width, [$($base),*], (attr_dec $attr));
    }
}

#[macro_export]
macro_rules! fuzz_multi_attr_bin {
    ($ctx:ident, $attr:expr, $width:expr, [$($base:tt),*]) => {
        $crate::fuzz_multi!($ctx, $attr, "", $width, [$($base),*], (attr_bin $attr));
    }
}

#[macro_export]
macro_rules! fuzz_multi_attr_hex {
    ($ctx:ident, $attr:expr, $width:expr, [$($base:tt),*]) => {
        $crate::fuzz_multi!($ctx, $attr, "", $width, [$($base),*], (attr_hex $attr));
    }
}

#[macro_export]
macro_rules! fuzz_enum {
    ($ctx:ident, $attr:expr, $vals:expr, [$($base:tt),*]) => {
        for val in $vals {
            $crate::fuzz_one!($ctx, $attr, val, [$($base),*], [(attr $attr, val)]);
        }
    }
}

#[macro_export]
macro_rules! fuzz_inv {
    ($ctx:ident, $pin:expr, [$($base:tt),*]) => {
        $crate::fuzz_enum!($ctx, &*format!("{}INV", $pin).leak(), [$pin, &*format!("{}_B", $pin).leak()], [(pin $pin), $($base),*]);
    }
}
