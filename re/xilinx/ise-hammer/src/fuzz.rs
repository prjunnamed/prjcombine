use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::{BelId, NodeKindId};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{backend::IseBackend, fgen::TileBits};

pub struct FuzzCtx<'sm, 's, 'a, 'b> {
    pub session: &'sm mut Session<'s, IseBackend<'a>>,
    pub backend: &'b IseBackend<'a>,
    pub node_kind: NodeKindId,
    pub bits: TileBits,
    pub tile_name: String,
    pub bel: BelId,
    pub bel_name: String,
}

impl<'sm, 's, 'a, 'b> FuzzCtx<'sm, 's, 'a, 'b> {
    pub fn new(
        session: &'sm mut Session<'s, IseBackend<'a>>,
        backend: &'b IseBackend<'a>,
        tile: impl Into<String>,
        bel: impl Into<String>,
        bits: TileBits,
    ) -> Self {
        let tile = tile.into();
        let node_kind = backend.egrid.db.get_node(&tile);
        let bel_name = bel.into();
        Self {
            session,
            backend,
            node_kind,
            bits,
            tile_name: tile,
            bel: backend.egrid.db.nodes[node_kind]
                .bels
                .get(&bel_name)
                .unwrap()
                .0,
            bel_name,
        }
    }

    pub fn try_new(
        session: &'sm mut Session<'s, IseBackend<'a>>,
        backend: &'b IseBackend<'a>,
        tile: impl Into<String>,
        bel: impl Into<String>,
        bits: TileBits,
    ) -> Option<Self> {
        let tile = tile.into();
        let node_kind = backend.egrid.db.get_node(&tile);
        if backend.egrid.node_index[node_kind].is_empty() {
            return None;
        }
        let bel_name = bel.into();
        Some(Self {
            session,
            backend,
            node_kind,
            bits,
            tile_name: tile,
            bel: backend.egrid.db.nodes[node_kind].bels.get(&bel_name)?.0,
            bel_name,
        })
    }

    pub fn new_force_bel(
        session: &'sm mut Session<'s, IseBackend<'a>>,
        backend: &'b IseBackend<'a>,
        tile: impl Into<String>,
        bel: impl Into<String>,
        bits: TileBits,
        bel_id: BelId,
    ) -> Self {
        let mut res = Self::new_fake_bel(session, backend, tile, bel, bits);
        res.bel = bel_id;
        res
    }

    pub fn new_fake_bel(
        session: &'sm mut Session<'s, IseBackend<'a>>,
        backend: &'b IseBackend<'a>,
        tile: impl Into<String>,
        bel: impl Into<String>,
        bits: TileBits,
    ) -> Self {
        let tile = tile.into();
        let node_kind = backend.egrid.db.get_node(&tile);
        let bel_name = bel.into();
        Self {
            session,
            backend,
            node_kind,
            bits,
            tile_name: tile,
            bel: BelId::from_idx(0),
            bel_name,
        }
    }

    pub fn new_fake_tile(
        session: &'sm mut Session<'s, IseBackend<'a>>,
        backend: &'b IseBackend<'a>,
        tile: impl Into<String>,
        bel: impl Into<String>,
        bits: TileBits,
    ) -> Self {
        let node_kind = backend.egrid.db.get_node(match backend.edev {
            ExpandedDevice::Xc2000(_) => "CNR.BL",
            ExpandedDevice::Virtex(_) => "CNR.BL",
            ExpandedDevice::Virtex2(edev) => match edev.grid.kind {
                prjcombine_virtex2::grid::GridKind::Virtex2 => "LL.V2",
                prjcombine_virtex2::grid::GridKind::Virtex2P
                | prjcombine_virtex2::grid::GridKind::Virtex2PX => "LL.V2P",
                prjcombine_virtex2::grid::GridKind::Spartan3 => "LL.S3",
                prjcombine_virtex2::grid::GridKind::FpgaCore => "LL.FC",
                prjcombine_virtex2::grid::GridKind::Spartan3E => "LL.S3E",
                prjcombine_virtex2::grid::GridKind::Spartan3A
                | prjcombine_virtex2::grid::GridKind::Spartan3ADsp => "LL.S3A",
            },
            ExpandedDevice::Spartan6(_) => "LL",
            ExpandedDevice::Virtex4(_) => "CFG",
            _ => todo!(),
        });
        let tile = tile.into();
        let bel_name = bel.into();
        Self {
            session,
            backend,
            node_kind,
            bits,
            tile_name: tile,
            bel: BelId::from_idx(0),
            bel_name,
        }
    }
}

#[macro_export]
macro_rules! fuzz_wire {
    ($ctx:ident, (int $tile:expr, $wire:expr)) => {
        $crate::fgen::TileWire::IntWire((
            <prjcombine_interconnect::db::NodeTileId as unnamed_entity::EntityId>::from_idx($tile),
            $ctx.backend.egrid.db.get_wire(&$wire[..]),
        ))
    };
    ($ctx:ident, (pin $pin:expr)) => {
        $crate::fgen::TileWire::BelPinNear($ctx.bel, $pin.to_string())
    };
    ($ctx:ident, (pin_far $pin:expr)) => {
        $crate::fgen::TileWire::BelPinFar($ctx.bel, $pin.to_string())
    };
    ($ctx:ident, (bel_pin $bel:expr, $pin:expr)) => {
        $crate::fgen::TileWire::BelPinNear($bel, $pin.to_string())
    };
    ($ctx:ident, (bel_pin_far $bel:expr, $pin:expr)) => {
        $crate::fgen::TileWire::BelPinFar($bel, $pin.to_string())
    };
    ($ctx:ident, (related_pin $relation:expr, $pin:expr)) => {
        $crate::fgen::TileWire::RelatedBelPinNear($ctx.bel, $relation, $pin.to_string())
    };
}

#[macro_export]
macro_rules! fuzz_base {
    ($ctx:ident, (mode $kind:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Mode($kind.to_string()))
    };
    ($ctx:ident, (bel_mode $bel:expr, $kind:expr)) => {
        $crate::fgen::TileKV::Bel($bel, $crate::fgen::BelKV::Mode($kind.to_string()))
    };
    ($ctx:ident, (unused)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Unused)
    };
    ($ctx:ident, (bel_unused $bel:expr)) => {
        $crate::fgen::TileKV::Bel($bel, $crate::fgen::BelKV::Unused)
    };
    ($ctx:ident, (iob_mode $iob:expr, $kind:expr)) => {
        $crate::fgen::TileKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelKV::Mode($kind.to_string()),
        )
    };
    ($ctx:ident, (attr $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::Attr($attr.to_string(), $val.to_string()),
        )
    };
    ($ctx:ident, (attr_any $attr:expr, $vals:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::AttrAny(
                $attr.to_string(),
                $vals.into_iter().map(|x| x.into()).collect(),
            ),
        )
    };
    ($ctx:ident, (bel_attr $bel:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel(
            $bel,
            $crate::fgen::BelKV::Attr($attr.to_string(), $val.to_string()),
        )
    };
    ($ctx:ident, (iob_attr $iob:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelKV::Attr($attr.to_string(), $val.to_string()),
        )
    };
    ($ctx:ident, (pin $pin:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Pin($pin.to_string(), true))
    };
    ($ctx:ident, (pin_from $pin:expr, $kind:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::PinFrom($pin.to_string(), $kind),
        )
    };
    ($ctx:ident, (pin_pips $pin:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::PinPips($pin.to_string()))
    };
    ($ctx:ident, (pin_node_mutex_shared $pin:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::PinNodeMutexShared($pin.to_string()),
        )
    };
    ($ctx:ident, (iob_pin $iob:expr, $pin:expr)) => {
        $crate::fgen::TileKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelKV::Pin($pin.to_string(), true),
        )
    };
    ($ctx:ident, (nopin $pin:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $crate::fgen::BelKV::Pin($pin.to_string(), false))
    };
    ($ctx:ident, (bel_pin $bel:expr, $pin:expr)) => {
        $crate::fgen::TileKV::Bel($bel, $crate::fgen::BelKV::Pin($pin.to_string(), true))
    };
    ($ctx:ident, (pin_pair $pina:expr, $bel:expr, $pinb:expr)) => {
        $crate::fgen::TileKV::PinPair($ctx.bel, $pina.to_string(), $bel, $pinb.to_string())
    };
    ($ctx:ident, (bel_special $special:expr)) => {
        $crate::fgen::TileKV::Bel($ctx.bel, $special)
    };
    ($ctx:ident, (iob_special $iob:expr, $special:expr)) => {
        $crate::fgen::TileKV::IobBel($iob.tile, $iob.bel, $special)
    };
    ($ctx:ident, (mutex $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::Mutex($attr.to_string(), $val.to_string()),
        )
    };
    ($ctx:ident, (bel_mutex $bel:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel(
            $bel,
            $crate::fgen::BelKV::Mutex($attr.to_string(), $val.to_string()),
        )
    };
    ($ctx:ident, (tile_mutex $attr:expr, $val:expr)) => {
        $crate::fgen::TileKV::TileMutex($attr.into(), $val.into())
    };
    ($ctx:ident, (package $pkg:expr)) => {
        $crate::fgen::TileKV::Package($pkg.to_string())
    };
    ($ctx:ident, (vccaux $val:expr)) => {
        $crate::fgen::TileKV::VccAux($val.to_string())
    };
    ($ctx:ident, (global_opt $name:expr, $val:expr)) => {
        $crate::fgen::TileKV::GlobalOpt($name.into(), $val.into())
    };
    ($ctx:ident, (no_global_opt $name:expr)) => {
        $crate::fgen::TileKV::NoGlobalOpt($name.into())
    };
    ($ctx:ident, (global_opt_any $attr:expr, $vals:expr)) => {
        $crate::fgen::TileKV::GlobalOptAny(
            $attr.to_string(),
            $vals.into_iter().map(|x| x.into()).collect(),
        )
    };
    ($ctx:ident, (global_xy $opt:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::Global(
                $crate::fgen::BelGlobalKind::Xy,
                $opt.to_string(),
                $val.to_string(),
            ),
        )
    };
    ($ctx:ident, (global_dll $opt:expr, $val:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::Global(
                $crate::fgen::BelGlobalKind::Dll,
                $opt.to_string(),
                $val.to_string(),
            ),
        )
    };
    ($ctx:ident, (global_mutex_none $name:expr)) => {
        $crate::fgen::TileKV::GlobalMutexNone($name.into())
    };
    ($ctx:ident, (global_mutex $name:expr, $val:expr)) => {
        $crate::fgen::TileKV::GlobalMutex($name.into(), $val.into())
    };
    ($ctx:ident, (global_mutex_site $name:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::GlobalMutexHere($name.to_string()),
        )
    };
    ($ctx:ident, (row_mutex $name:expr, $val:expr)) => {
        $crate::fgen::TileKV::RowMutex($name.into(), $val.into())
    };
    ($ctx:ident, (row_mutex_site $name:expr)) => {
        $crate::fgen::TileKV::Bel(
            $ctx.bel,
            $crate::fgen::BelKV::RowMutexHere($name.to_string()),
        )
    };
    ($ctx:ident, (pip $wa:tt, $wb:tt)) => {
        $crate::fgen::TileKV::Pip($crate::fuzz_wire!($ctx, $wa), $crate::fuzz_wire!($ctx, $wb))
    };
    ($ctx:ident, (related $rel:expr, $inner:tt)) => {
        $crate::fgen::TileKV::TileRelated($rel, Box::new($crate::fuzz_base!($ctx, $inner)))
    };
    ($ctx:ident, (no_related $rel:expr)) => {
        $crate::fgen::TileKV::NoTileRelated($rel)
    };
    ($ctx:ident, (special $val:expr)) => {
        $val
    };
    ($ctx:ident, (nop)) => {
        $crate::fgen::TileKV::Nop
    };
    ($ctx:ident, (raw $key:expr, $val:expr)) => {
        $crate::fgen::TileKV::Raw($key, $val.into())
    };
}

#[macro_export]
macro_rules! fuzz_diff {
    ($ctx:ident, (mode $kind:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $crate::fgen::BelFuzzKV::Mode($kind.to_string()))
    };
    ($ctx:ident, (mode_diff $kinda:expr, $kindb:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $ctx.bel,
            $crate::fgen::BelFuzzKV::ModeDiff($kinda.to_string(), $kindb.to_string()),
        )
    };
    ($ctx:ident, (bel_mode_diff $bel:expr, $kinda:expr, $kindb:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $bel,
            $crate::fgen::BelFuzzKV::ModeDiff($kinda.to_string(), $kindb.to_string()),
        )
    };
    ($ctx:ident, (attr $attr:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $ctx.bel,
            $crate::fgen::BelFuzzKV::Attr($attr.to_string(), $val.to_string()),
        )
    };
    ($ctx:ident, (bel_attr $bel:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $bel,
            $crate::fgen::BelFuzzKV::Attr($attr.to_string(), $val.to_string()),
        )
    };
    ($ctx:ident, (attr_diff $attr:expr, $vala:expr, $valb:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $ctx.bel,
            $crate::fgen::BelFuzzKV::AttrDiff(
                $attr.to_string(),
                $vala.to_string(),
                $valb.to_string(),
            ),
        )
    };
    ($ctx:ident, (bel_attr_diff $bel:expr, $attr:expr, $vala:expr, $valb:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $bel,
            $crate::fgen::BelFuzzKV::AttrDiff(
                $attr.to_string(),
                $vala.to_string(),
                $valb.to_string(),
            ),
        )
    };
    ($ctx:ident, (pin $pin:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $crate::fgen::BelFuzzKV::Pin($pin.to_string()))
    };
    ($ctx:ident, (bel_pin $bel:expr, $pin:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($bel, $crate::fgen::BelFuzzKV::Pin($pin.to_string()))
    };
    ($ctx:ident, (pin_full $pin:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $crate::fgen::BelFuzzKV::PinFull($pin.to_string()))
    };
    ($ctx:ident, (pin_pips $pin:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $crate::fgen::BelFuzzKV::PinPips($pin.to_string()))
    };
    ($ctx:ident, (pin_from $pin:expr, $kind_a:expr, $kind_b:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $ctx.bel,
            $crate::fgen::BelFuzzKV::PinFrom($pin.to_string(), $kind_a, $kind_b),
        )
    };
    ($ctx:ident, (pin_pair $pina:expr, $bel:expr, $pinb:expr)) => {
        $crate::fgen::TileFuzzKV::PinPair($ctx.bel, $pina.to_string(), $bel, $pinb.to_string())
    };
    ($ctx:ident, (iob_mode $iob:expr, $kind:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelFuzzKV::Mode($kind.to_string()),
        )
    };
    ($ctx:ident, (iob_mode_diff $iob:expr, $kinda:expr, $kindb:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelFuzzKV::ModeDiff($kinda.to_string(), $kindb.to_string()),
        )
    };
    ($ctx:ident, (iob_attr $iob:expr, $attr:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelFuzzKV::Attr($attr.to_string(), $val.to_string()),
        )
    };
    ($ctx:ident, (iob_attr_diff $iob:expr, $attr:expr, $vala:expr, $valb:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelFuzzKV::AttrDiff(
                $attr.to_string(),
                $vala.to_string(),
                $valb.to_string(),
            ),
        )
    };
    ($ctx:ident, (iob_pin $iob:expr, $pin:expr)) => {
        $crate::fgen::TileFuzzKV::IobBel(
            $iob.tile,
            $iob.bel,
            $crate::fgen::BelFuzzKV::Pin($pin.to_string()),
        )
    };
    ($ctx:ident, (global_opt $opt:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::GlobalOpt($opt.to_string(), $val.to_string())
    };
    ($ctx:ident, (global_xy $opt:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $ctx.bel,
            $crate::fgen::BelFuzzKV::Global(
                $crate::fgen::BelGlobalKind::Xy,
                $opt.to_string(),
                $val.to_string(),
            ),
        )
    };
    ($ctx:ident, (global_dll $opt:expr, $val:expr)) => {
        $crate::fgen::TileFuzzKV::Bel(
            $ctx.bel,
            $crate::fgen::BelFuzzKV::Global(
                $crate::fgen::BelGlobalKind::Dll,
                $opt.to_string(),
                $val.to_string(),
            ),
        )
    };
    ($ctx:ident, (global_opt_diff $opt:expr, $vala:expr, $valb:expr)) => {
        $crate::fgen::TileFuzzKV::GlobalOptDiff(
            $opt.to_string(),
            $vala.to_string(),
            $valb.to_string(),
        )
    };
    ($ctx:ident, (pip $wa:tt, $wb:tt)) => {
        $crate::fgen::TileFuzzKV::Pip($crate::fuzz_wire!($ctx, $wa), $crate::fuzz_wire!($ctx, $wb))
    };
    ($ctx:ident, (row_mutex $name:expr)) => {
        $crate::fgen::TileFuzzKV::RowMutexExclusive($name.to_string())
    };
    ($ctx:ident, (tile_mutex $attr:expr)) => {
        $crate::fgen::TileFuzzKV::TileMutexExclusive($attr.into())
    };
    ($ctx:ident, (related $rel:expr, $inner:tt)) => {
        $crate::fgen::TileFuzzKV::TileRelated($rel, Box::new($crate::fuzz_diff!($ctx, $inner)))
    };
    ($ctx:ident, (special $val:expr)) => {
        $val
    };
    ($ctx:ident, (bel_special $special:expr)) => {
        $crate::fgen::TileFuzzKV::Bel($ctx.bel, $special)
    };
    ($ctx:ident, (raw $key:expr, $val0:expr, $val1:expr)) => {
        $crate::fgen::TileFuzzKV::Raw($key, $val0.into(), $val1.into())
    };
}

#[macro_export]
macro_rules! fuzz_diff_multi {
    ($ctx:ident, (attr_lut $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr.to_string(),
            $crate::backend::MultiValue::Lut,
        )
    };
    ($ctx:ident, (attr_oldlut $attr:expr, $f:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr.to_string(),
            $crate::backend::MultiValue::OldLut($f),
        )
    };
    ($ctx:ident, (attr_hex $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr.into(),
            $crate::backend::MultiValue::Hex(0),
        )
    };
    ($ctx:ident, (attr_hex_delta $attr:expr, $delta:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr.into(),
            $crate::backend::MultiValue::Hex($delta),
        )
    };
    ($ctx:ident, (attr_bin $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr.to_string(),
            $crate::backend::MultiValue::Bin,
        )
    };
    ($ctx:ident, (attr_dec $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr.to_string(),
            $crate::backend::MultiValue::Dec(0),
        )
    };
    ($ctx:ident, (attr_dec_delta $attr:expr, $delta:expr)) => {
        $crate::fgen::TileMultiFuzzKV::SiteAttr(
            $ctx.bel,
            $attr.to_string(),
            $crate::backend::MultiValue::Dec($delta),
        )
    };
    ($ctx:ident, (iob_attr_bin $iob:expr, $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::IobSiteAttr(
            $iob.tile,
            $iob.bel,
            $attr.to_string(),
            $crate::backend::MultiValue::Bin,
        )
    };
    ($ctx:ident, (global_hex $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::GlobalOpt(
            $attr.to_string(),
            $crate::backend::MultiValue::Hex(0),
        )
    };
    ($ctx:ident, (global_hex_prefix $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::GlobalOpt(
            $attr.to_string(),
            $crate::backend::MultiValue::HexPrefix,
        )
    };
    ($ctx:ident, (global_bin $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::GlobalOpt(
            $attr.to_string(),
            $crate::backend::MultiValue::Bin,
        )
    };
    ($ctx:ident, (global_dec $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::GlobalOpt(
            $attr.to_string(),
            $crate::backend::MultiValue::Dec(0),
        )
    };
    ($ctx:ident, (global_xy_bin $attr:expr)) => {
        $crate::fgen::TileMultiFuzzKV::BelGlobalOpt(
            $ctx.bel,
            $crate::fgen::BelGlobalKind::Xy,
            $attr.to_string(),
            $crate::backend::MultiValue::Bin,
        )
    };
    ($ctx:ident, (raw $key:expr, $val:expr)) => {
        $crate::fgen::TileMultiFuzzKV::Raw($key, $val)
    };
}

#[macro_export]
macro_rules! fuzz_one_extras {
    ($ctx:ident, $attr:expr, $val:expr, [$($base:tt),*], [$($diff:tt),*], $extras:expr) => {
        $ctx.session.add_fuzzer(Box::new($crate::fgen::TileFuzzerGen {
            node: $ctx.node_kind,
            bits: $ctx.bits.clone(),
            feature: prjcombine_re_collector::FeatureId {
                tile: $ctx.tile_name.clone(),
                bel: $ctx.bel_name.clone(),
                attr: $attr.into(),
                val: $val.into(),
            },
            base: vec![
                $($crate::fuzz_base!($ctx, $base)),*
            ],
            fuzz: vec![
                $($crate::fuzz_diff!($ctx, $diff)),*
            ],
            extras: $extras,
        }));
    };
}

#[macro_export]
macro_rules! fuzz_one {
    ($ctx:ident, $attr:expr, $val:expr, [$($base:tt),*], [$($diff:tt),*]) => {
        $crate::fuzz_one_extras!($ctx, $attr, $val, [$($base),*], [$($diff),*], vec![]);
    };
}

#[macro_export]
macro_rules! fuzz_multi_extras {
    ($ctx:ident, $attr:expr, $val:expr, $width:expr, [$($base:tt),*], $diff:tt, $extras:expr) => {
        $ctx.session.add_fuzzer(Box::new($crate::fgen::TileMultiFuzzerGen {
            node: $ctx.node_kind,
            bits: $ctx.bits.clone(),
            feature: prjcombine_re_collector::FeatureId {
                tile: $ctx.tile_name.clone(),
                bel: $ctx.bel_name.clone(),
                attr: $attr.to_string(),
                val: $val.to_string(),
            },
            base: vec![
                $($crate::fuzz_base!($ctx, $base)),*
            ],
            width: $width,
            fuzz: $crate::fuzz_diff_multi!($ctx, $diff),
            extras: $extras,
        }));
    };
}

#[macro_export]
macro_rules! fuzz_multi {
    ($ctx:ident, $attr:expr, $val:expr, $width:expr, [$($base:tt),*], $diff:tt) => {
        $crate::fuzz_multi_extras!($ctx, $attr, $val, $width, [$($base),*], $diff, vec![]);
    };
}

#[macro_export]
macro_rules! fuzz_multi_attr_dec {
    ($ctx:ident, $attr:expr, $width:expr, [$($base:tt),*]) => {
        $crate::fuzz_multi!($ctx, $attr, "", $width, [$($base),*], (attr_dec $attr));
    }
}

#[macro_export]
macro_rules! fuzz_multi_attr_dec_delta {
    ($ctx:ident, $attr:expr, $width:expr, $delta:expr, [$($base:tt),*]) => {
        $crate::fuzz_multi!($ctx, $attr, "", $width, [$($base),*], (attr_dec_delta $attr, $delta));
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
macro_rules! fuzz_enum_suffix {
    ($ctx:ident, $attr:expr, $suffix:expr, $vals:expr, [$($base:tt),*]) => {
        for val in $vals {
            $crate::fuzz_one!($ctx, format!("{}.{}", $attr, $suffix), val, [$($base),*], [(attr $attr, val)]);
        }
    }
}

#[macro_export]
macro_rules! fuzz_inv {
    ($ctx:ident, $pin:expr, [$($base:tt),*]) => {
        let pininv = format!("{}INV", $pin);
        let pin_b = format!("{}_B", $pin);
        $crate::fuzz_enum!($ctx, &pininv, [$pin, &pin_b], [(pin $pin), $($base),*]);
    }
}

#[macro_export]
macro_rules! fuzz_inv_suffix {
    ($ctx:ident, $pin:expr, $suffix:expr, [$($base:tt),*]) => {
        let pininv = format!("{}INV", $pin);
        let pin_b = format!("{}_B", $pin);
        $crate::fuzz_enum_suffix!($ctx, &pininv, $suffix, [$pin, &pin_b], [(pin $pin), $($base),*]);
    }
}
