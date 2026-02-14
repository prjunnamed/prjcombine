use std::collections::BTreeSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttributeType, BelInputId, TileWireCoord, WireSlotIdExt},
    grid::{DieId, TileCoord},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, SpecialId, xlat_bit, xlat_bit_wide, xlat_enum_attr, xlat_enum_raw,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bsdata::TileBit};
use prjcombine_virtex4::defs::{
    self, bcls, bslots, enums,
    virtex4::{tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{
            DynProp,
            mutex::WireMutexExclusive,
            relation::{Delta, TileRelation},
        },
    },
    virtex4::specials,
};

const GT11_INVPINS: &[BelInputId] = &[
    bcls::GT11::DCLK,
    bcls::GT11::DEN,
    bcls::GT11::DWE,
    bcls::GT11::RXCRCCLK,
    bcls::GT11::RXCRCDATAVALID,
    bcls::GT11::RXCRCINTCLK,
    bcls::GT11::RXCRCRESET,
    bcls::GT11::RXPMARESET,
    bcls::GT11::RXRESET,
    bcls::GT11::RXUSRCLK2,
    bcls::GT11::RXUSRCLK,
    bcls::GT11::SCANEN.index_const(0),
    bcls::GT11::SCANEN.index_const(1),
    bcls::GT11::SCANEN.index_const(2),
    bcls::GT11::SCANMODE.index_const(0),
    bcls::GT11::SCANMODE.index_const(1),
    bcls::GT11::SCANMODE.index_const(2),
    bcls::GT11::TXCRCCLK,
    bcls::GT11::TXCRCDATAVALID,
    bcls::GT11::TXCRCINTCLK,
    bcls::GT11::TXCRCRESET,
    bcls::GT11::TXPMARESET,
    bcls::GT11::TXRESET,
    bcls::GT11::TXUSRCLK2,
    bcls::GT11::TXUSRCLK,
];

#[derive(Clone, Debug)]
struct MgtRepeaterMgt(i32, TileWireCoord, SpecialId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for MgtRepeaterMgt {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let row = tcrd.row + self.0;
        let is_w = tcrd.col < edev.col_cfg;
        for &col in &edev.chips[tcrd.die].cols_vbrk {
            if (col < edev.col_cfg) == is_w {
                let rcol = if is_w { col } else { col - 1 };
                let ntcrd = tcrd.with_cr(rcol, row).tile(defs::tslots::CLK);
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::RoutingSpecial(tcls::HCLK_MGT_BUF, self.1, self.2),
                    rects: edev.tile_bits(ntcrd),
                });
            }
        }

        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct ClkHrow(i32);

impl TileRelation for ClkHrow {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(
            tcrd.cell
                .with_col(edev.col_clk)
                .delta(0, self.0)
                .tile(defs::tslots::HROW),
        )
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::MGT) else {
        return;
    };
    for i in 0..2 {
        let bel = format!("GT11[{i}]");
        let mut bctx = ctx.bel(defs::bslots::GT11[i]);
        let mode = "GT11";
        bctx.build()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for &pin in GT11_INVPINS {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }

        for (aid, aname, attr) in &backend.edev.db[bcls::GT11].attributes {
            match aid {
                bcls::GT11::DRP | bcls::GT11::DRP_MASK => (),

                bcls::GT11::COMMA_10B_MASK
                | bcls::GT11::RESERVED_CM
                | bcls::GT11::RESERVED_CM2
                | bcls::GT11::RXCRCINITVAL
                | bcls::GT11::RXCTRL1
                | bcls::GT11::RXEQ
                | bcls::GT11::RXTUNE
                | bcls::GT11::TXCRCINITVAL
                | bcls::GT11::TXLNDR_TST3 => {
                    let BelAttributeType::BitVec(width) = attr.typ else {
                        unreachable!()
                    };
                    bctx.mode(mode).test_bel_attr_bits(aid).multi_attr(
                        aname,
                        MultiValue::Hex(0),
                        width,
                    );
                }

                bcls::GT11::CHAN_BOND_LIMIT
                | bcls::GT11::CLK_COR_MIN_LAT
                | bcls::GT11::CLK_COR_MAX_LAT
                | bcls::GT11::SH_INVALID_CNT_MAX
                | bcls::GT11::SH_CNT_MAX => {
                    let BelAttributeType::BitVec(width) = attr.typ else {
                        unreachable!()
                    };
                    bctx.mode(mode).test_bel_attr_bits(aid).multi_attr(
                        aname,
                        MultiValue::Dec(0),
                        width,
                    );
                }

                bcls::GT11::MCOMMA_VALUE => {
                    bctx.mode(mode)
                        .attr("MCOMMA_32B_VALUE", "")
                        .test_bel_attr_bits(aid)
                        .multi_attr("MCOMMA_10B_VALUE", MultiValue::Hex(0), 10);
                    bctx.mode(mode)
                        .attr("MCOMMA_10B_VALUE", "")
                        .test_bel_attr_bits(aid)
                        .multi_attr("MCOMMA_32B_VALUE", MultiValue::Hex(0), 32);
                }

                bcls::GT11::PCOMMA_VALUE => {
                    bctx.mode(mode)
                        .attr("PCOMMA_32B_VALUE", "")
                        .test_bel_attr_bits(aid)
                        .multi_attr("PCOMMA_10B_VALUE", MultiValue::Hex(0), 10);
                    bctx.mode(mode)
                        .attr("PCOMMA_10B_VALUE", "")
                        .test_bel_attr_bits(aid)
                        .multi_attr("PCOMMA_32B_VALUE", MultiValue::Hex(0), 32);
                }

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::BitVec(width) => {
                        bctx.mode(mode).test_bel_attr_bits(aid).multi_attr(
                            aname,
                            MultiValue::Bin,
                            width,
                        );
                    }
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode).test_bel_attr_auto(aid);
                    }
                    _ => unreachable!(),
                },
            }
        }

        for val in ["FALSE", "TRUE"] {
            bctx.mode(mode)
                .null_bits()
                .test_bel_special(specials::GT11_PMACLKENABLE)
                .attr("PMACLKENABLE", val)
                .commit();
        }

        for val in ["SINGLE", "DONT_CARE", "B", "A"] {
            bctx.mode(mode)
                .null_bits()
                .test_bel_special(specials::GT11_MODE)
                .attr("GT11_MODE", val)
                .commit();
        }

        for (aid, aname, attr) in &backend.edev.db[bcls::GT11CLK].attributes {
            match aid {
                bcls::GT11CLK::REFCLKSEL
                | bcls::GT11CLK::SYNCLK1_DRIVE
                | bcls::GT11CLK::SYNCLK2_DRIVE
                | bcls::GT11CLK::SYNCLK_DRIVE_ENABLE
                | bcls::GT11CLK::SYNCLK_ENABLE => (),

                bcls::GT11CLK::ATBSEL
                | bcls::GT11CLK::PMACFG2SPARE
                | bcls::GT11CLK::TXCTRL1
                | bcls::GT11CLK::TXTUNE => {
                    let BelAttributeType::BitVec(width) = attr.typ else {
                        unreachable!()
                    };
                    bctx.mode(mode)
                        .tile_mutex(aname, &bel)
                        .test_bel(bslots::GT11CLK)
                        .test_bel_attr_bits(aid)
                        .multi_attr(aname, MultiValue::Hex(0), width);
                }

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .tile_mutex(aname, &bel)
                            .test_bel(bslots::GT11CLK)
                            .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::BitVec(width) => {
                        bctx.mode(mode)
                            .tile_mutex(aname, &bel)
                            .test_bel(bslots::GT11CLK)
                            .test_bel_attr_bits(aid)
                            .multi_attr(aname, MultiValue::Bin, width);
                    }
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode)
                            .tile_mutex(aname, &bel)
                            .test_bel(bslots::GT11CLK)
                            .test_bel_attr_auto(aid);
                    }
                    _ => unreachable!(),
                },
            }
        }

        let hclk_cell = match i {
            0 => 8,
            1 => 24,
            _ => unreachable!(),
        };
        for (pin, wt) in [
            ("REFCLK", wires::IMUX_MGT_REFCLK_PRE[i]),
            ("PMACLK", wires::IMUX_MGT_GREFCLK_PRE[i]),
        ] {
            for j in 0..8 {
                let wt = wt.cell(16);
                let wf = wires::HCLK_MGT[j].cell(hclk_cell);
                bctx.build()
                    .mutex("HCLK_IN", format!("HCLK{j}"))
                    .mutex("HCLK_OUT", pin)
                    .global_mutex("BUFGCTRL_OUT", "USE")
                    .related_tile_mutex(ClkHrow(hclk_cell as i32), "MODE", "USE")
                    .related_pip(
                        ClkHrow(hclk_cell as i32),
                        wires::HCLK_ROW[j].cell(0),
                        wires::GCLK[0].cell(0),
                    )
                    .related_pip(
                        ClkHrow(hclk_cell as i32),
                        wires::HCLK_ROW[j].cell(1),
                        wires::GCLK[0].cell(0),
                    )
                    .extra_tile_routing(
                        Delta::new(0, hclk_cell as i32, tcls::HCLK_MGT),
                        wires::HCLK_MGT[j].cell(0),
                        wires::HCLK_ROW[j].cell(0).pos(),
                    )
                    .test_routing(wt, wf.pos())
                    .prop(FuzzIntPip::new(wt, wf))
                    .commit();
            }
        }
        let c = 8 + i * 16;
        for o in 0..2 {
            let dst = wires::MGT_CLK_OUT[o].cell(c);
            for w in [
                wires::MGT_CLK_OUT_SYNCLK,
                wires::MGT_CLK_OUT_FWDCLK[0],
                wires::MGT_CLK_OUT_FWDCLK[1],
            ] {
                let src = w.cell(c);
                bctx.build()
                    .global_mutex("MGT_OUT", "TEST")
                    .tile_mutex("SYNCLK", "USE")
                    .mutex("SYNCLK_OUT", "USE")
                    .prop(BaseIntPip::new(
                        wires::MGT_CLK_OUT_SYNCLK.cell(c),
                        wires::OUT_MGT_SYNCLK[0].cell(16),
                    ))
                    .extra_tile_routing(
                        Delta::new(0, hclk_cell as i32, tcls::HCLK_MGT),
                        wires::MGT_ROW[o].cell(0),
                        wires::MGT_CLK_OUT[o].cell(0).pos(),
                    )
                    .prop(MgtRepeaterMgt(
                        hclk_cell as i32,
                        wires::MGT_ROW[o].cell(0),
                        specials::MGT_BUF_MGT,
                    ))
                    .test_routing(dst, src.pos())
                    .prop(WireMutexExclusive::new(dst))
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            }
        }
        for w in wires::OUT_MGT_SYNCLK {
            let dst = wires::MGT_CLK_OUT_SYNCLK.cell(c);
            let src = w.cell(16);
            bctx.build()
                .tile_mutex("SYNCLK", &bel)
                .mutex("SYNCLK_OUT", "DRIVE")
                .test_routing(dst, src.pos())
                .prop(WireMutexExclusive::new(dst))
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
        for w in wires::MGT_CLK_OUT_FWDCLK {
            let dst = w.cell(c);
            let mux = &backend.edev.db_index.tile_classes[tcls::MGT].muxes[&dst];
            for &src in mux.src.keys() {
                bctx.build()
                    .tile_mutex("FWDCLK_MUX", dst)
                    .test_routing(dst, src)
                    .prop(WireMutexExclusive::new(dst))
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            }
        }
        let (w, wo) = match i {
            0 => (wires::MGT_FWDCLK_S, wires::MGT_FWDCLK_N),
            1 => (wires::MGT_FWDCLK_N, wires::MGT_FWDCLK_S),
            _ => unreachable!(),
        };
        for j in 0..4 {
            let dst = w[j].cell(16);
            for src in [
                wires::OUT_MGT_RXPCSHCLKOUT[0].cell(16),
                wires::OUT_MGT_RXPCSHCLKOUT[1].cell(16),
                wires::OUT_MGT_TXPCSHCLKOUT[0].cell(16),
                wires::OUT_MGT_TXPCSHCLKOUT[1].cell(16),
            ] {
                bctx.build()
                    .global_mutex("MGT_FWDCLK_BUF", "DRIVE")
                    .test_routing(dst, src.pos())
                    .prop(WireMutexExclusive::new(dst))
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            }
            let src = wo[j].cell(16);
            let help = wires::OUT_MGT_RXPCSHCLKOUT[0].cell(16);
            bctx.build()
                .global_mutex("MGT_FWDCLK_BUF", "DRIVE")
                .prop(WireMutexExclusive::new(src))
                .prop(BaseIntPip::new(src, help))
                .test_routing(dst, src.pos())
                .prop(WireMutexExclusive::new(dst))
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
    }

    let mut bctx = ctx.bel(defs::bslots::GT11CLK);
    let mode = "GT11CLK";
    bctx.build()
        .null_bits()
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();
    bctx.mode(mode).test_bel_attr_auto(bcls::GT11CLK::REFCLKSEL);

    for (i, attr) in [
        (1, bcls::GT11CLK::SYNCLK1_DRIVE),
        (2, bcls::GT11CLK::SYNCLK2_DRIVE),
    ] {
        bctx.build()
            .global_mutex("SYNCLK_BUF_DIR", "UP")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_BUF_UP"))
            .related_pip(
                Delta::new(0, -32, tcls::MGT),
                format!("SYNCLK{i}"),
                format!("SYNCLK{i}OUT"),
            )
            .related_tile_mutex(Delta::new(0, -32, tcls::MGT), "SYNCLK", "USE")
            .test_bel_attr_val(attr, enums::GT11_SYNCLK_DRIVE::BUF_UP)
            .pip(format!("SYNCLK{i}"), format!("SYNCLK{i}_S"))
            .commit();
        bctx.build()
            .global_mutex("SYNCLK_BUF_DIR", "DOWN")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_BUF_DOWN"))
            .related_pip(
                Delta::new(0, 32, tcls::MGT),
                format!("SYNCLK{i}_S"),
                format!("SYNCLK{i}OUT"),
            )
            .related_tile_mutex(Delta::new(0, 32, tcls::MGT), "SYNCLK", "USE")
            .test_bel_attr_val(attr, enums::GT11_SYNCLK_DRIVE::BUF_DOWN)
            .pip(format!("SYNCLK{i}_S"), format!("SYNCLK{i}"))
            .commit();
        bctx.mode(mode)
            .global_mutex("SYNCLK_BUF_DIR", "UP")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_DRIVE_UP"))
            .test_bel_attr_val(attr, enums::GT11_SYNCLK_DRIVE::DRIVE_UP)
            .attr(format!("SYNCLK{i}OUTEN"), "ENABLE")
            .pin(format!("SYNCLK{i}OUT"))
            .pip(format!("SYNCLK{i}"), format!("SYNCLK{i}OUT"))
            .commit();
        bctx.mode(mode)
            .global_mutex("SYNCLK_BUF_DIR", "DOWN")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_DRIVE_DOWN"))
            .test_bel_attr_val(attr, enums::GT11_SYNCLK_DRIVE::DRIVE_DOWN)
            .attr(format!("SYNCLK{i}OUTEN"), "ENABLE")
            .pin(format!("SYNCLK{i}OUT"))
            .pip(format!("SYNCLK{i}_S"), format!("SYNCLK{i}OUT"))
            .commit();
        bctx.mode(mode)
            .global_mutex_here("SYNCLK_BUF_DIR")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_DRIVE_BOTH"))
            .test_bel_attr_val(attr, enums::GT11_SYNCLK_DRIVE::DRIVE_BOTH)
            .attr(format!("SYNCLK{i}OUTEN"), "ENABLE")
            .pin(format!("SYNCLK{i}OUT"))
            .pip(format!("SYNCLK{i}"), format!("SYNCLK{i}OUT"))
            .pip(format!("SYNCLK{i}_S"), format!("SYNCLK{i}OUT"))
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let tcid = tcls::MGT;
    if !ctx.has_tcls(tcid) {
        return;
    }
    fn drp_bit(bel: usize, idx: usize, bit: usize) -> TileBit {
        let tile = bel << 4 | (idx & 7) << 1 | (idx & 0x20) >> 5;
        let bit = bit + 1 + 20 * (idx >> 3 & 3);
        TileBit::new(tile, 19, bit)
    }
    let (_, _, synclk_enable) = Diff::split(
        ctx.peek_diff_routing(
            tcid,
            wires::MGT_CLK_OUT_SYNCLK.cell(8),
            wires::OUT_MGT_SYNCLK[0].cell(16).pos(),
        )
        .clone(),
        ctx.peek_diff_routing(
            tcid,
            wires::MGT_CLK_OUT_SYNCLK.cell(24),
            wires::OUT_MGT_SYNCLK[0].cell(16).pos(),
        )
        .clone(),
    );
    for idx in 0..2 {
        let bslot = bslots::GT11[idx];
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);

        let mut drp = vec![];
        let mut drp_mask = vec![];
        for i in 0x40..0x80 {
            for j in 0..16 {
                drp.push(drp_bit(idx, i, j).pos());
            }
            drp_mask.push(drp_bit(idx, i, 17).pos());
        }
        present.apply_bitvec_diff(&drp_mask, &bits![1; 64], &bits![0; 64]);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::GT11::DRP, drp);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::GT11::DRP_MASK, drp_mask);

        for &pin in GT11_INVPINS {
            ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 32], tcid, bslot, pin);
        }
        for pin in [
            bcls::GT11::RXRESET,
            bcls::GT11::RXCRCRESET,
            bcls::GT11::RXPMARESET,
            bcls::GT11::TXRESET,
            bcls::GT11::TXCRCRESET,
            bcls::GT11::TXPMARESET,
            bcls::GT11::RXCRCINTCLK,
            bcls::GT11::TXCRCINTCLK,
            bcls::GT11::RXCRCCLK,
            bcls::GT11::TXCRCCLK,
            bcls::GT11::RXCRCDATAVALID,
            bcls::GT11::TXCRCDATAVALID,
            bcls::GT11::DCLK,
            bcls::GT11::DEN,
            bcls::GT11::DWE,
        ] {
            present.apply_bit_diff(
                ctx.item_int_inv(&[tcls::INT; 32], tcid, bslot, pin),
                false,
                true,
            );
        }
        present.assert_empty();

        for (aid, _, attr) in &ctx.edev.db[bcls::GT11].attributes {
            match aid {
                bcls::GT11::DRP | bcls::GT11::DRP_MASK => (),

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        ctx.collect_bel_attr_bi(tcid, bslot, aid);
                    }
                    BelAttributeType::Enum(_) => {
                        ctx.collect_bel_attr_ocd(tcid, bslot, aid, OcdMode::BitOrder);
                    }
                    _ => {
                        ctx.collect_bel_attr(tcid, bslot, aid);
                    }
                },
            }
        }

        let c = 8 + idx * 16;
        let fwd = [wires::MGT_FWDCLK_S, wires::MGT_FWDCLK_N][idx];
        let (_, _, fwdclk_out_enable) = Diff::split(
            ctx.peek_diff_routing(
                tcid,
                wires::MGT_CLK_OUT_FWDCLK[0].cell(c),
                fwd[0].cell(16).pos(),
            )
            .clone(),
            ctx.peek_diff_routing(
                tcid,
                wires::MGT_CLK_OUT_FWDCLK[1].cell(c),
                fwd[0].cell(16).pos(),
            )
            .clone(),
        );
        for w in wires::MGT_CLK_OUT_FWDCLK {
            let dst = w.cell(c);
            let mut diffs = vec![];
            for wf in fwd {
                let src = wf.cell(16).pos();
                let mut diff = ctx.get_diff_routing(tcid, dst, src);
                diff = diff.combine(&!&fwdclk_out_enable);
                diffs.push((Some(src), diff));
            }
            ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::BitOrder));
        }
        ctx.insert_support(
            tcid,
            BTreeSet::from_iter([
                wires::MGT_CLK_OUT_FWDCLK[0].cell(8),
                wires::MGT_CLK_OUT_FWDCLK[1].cell(8),
                wires::MGT_CLK_OUT_FWDCLK[0].cell(24),
                wires::MGT_CLK_OUT_FWDCLK[1].cell(24),
            ]),
            vec![xlat_bit(fwdclk_out_enable)],
        );

        let c = 8 + idx * 16;
        for w in wires::MGT_CLK_OUT {
            ctx.collect_mux_ocd(tcid, w.cell(c), OcdMode::BitOrder);
        }
        let mut diffs = vec![(None, Default::default())];
        let dst = wires::MGT_CLK_OUT_SYNCLK.cell(c);
        for w in wires::OUT_MGT_SYNCLK {
            let src = w.cell(16).pos();
            let mut diff = ctx.get_diff_routing(tcid, dst, src);
            diff = diff.combine(&!&synclk_enable);
            diffs.push((Some(src), diff));
        }
        ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::BitOrder));
    }

    {
        let bslot = bslots::GT11CLK;
        for (aid, _, attr) in &ctx.edev.db[bcls::GT11CLK].attributes {
            match aid {
                bcls::GT11CLK::SYNCLK1_DRIVE => (),
                bcls::GT11CLK::SYNCLK2_DRIVE => (),
                bcls::GT11CLK::SYNCLK_DRIVE_ENABLE => (),
                bcls::GT11CLK::SYNCLK_ENABLE => (),

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        ctx.collect_bel_attr_bi(tcid, bslot, aid);
                    }
                    BelAttributeType::Enum(_) => {
                        ctx.collect_bel_attr_ocd(tcid, bslot, aid, OcdMode::BitOrder);
                    }
                    _ => {
                        ctx.collect_bel_attr(tcid, bslot, aid);
                    }
                },
            }
        }

        let (_, _, mut synclk_drive_enable) = Diff::split(
            ctx.peek_diff_attr_val(
                tcid,
                bslot,
                bcls::GT11CLK::SYNCLK1_DRIVE,
                enums::GT11_SYNCLK_DRIVE::BUF_DOWN,
            )
            .clone(),
            ctx.peek_diff_attr_val(
                tcid,
                bslot,
                bcls::GT11CLK::SYNCLK2_DRIVE,
                enums::GT11_SYNCLK_DRIVE::BUF_DOWN,
            )
            .clone(),
        );
        for attr in [bcls::GT11CLK::SYNCLK1_DRIVE, bcls::GT11CLK::SYNCLK2_DRIVE] {
            let mut diffs = vec![(enums::GT11_SYNCLK_DRIVE::NONE, Diff::default())];
            for val in [
                enums::GT11_SYNCLK_DRIVE::BUF_UP,
                enums::GT11_SYNCLK_DRIVE::BUF_DOWN,
                enums::GT11_SYNCLK_DRIVE::DRIVE_UP,
                enums::GT11_SYNCLK_DRIVE::DRIVE_DOWN,
                enums::GT11_SYNCLK_DRIVE::DRIVE_BOTH,
            ] {
                let mut diff = ctx.get_diff_attr_val(tcid, bslot, attr, val);
                diff = diff.combine(&!&synclk_drive_enable);
                diffs.push((val, diff));
            }
            ctx.insert_bel_attr_enum(tcid, bslot, attr, xlat_enum_attr(diffs));
        }
        synclk_drive_enable = synclk_drive_enable.combine(&!&synclk_enable);
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GT11CLK::SYNCLK_DRIVE_ENABLE,
            xlat_bit(synclk_drive_enable),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GT11CLK::SYNCLK_ENABLE,
            xlat_bit(synclk_enable),
        );
    }

    ctx.collect_mux(tcid, wires::IMUX_MGT_REFCLK_PRE[0].cell(16));
    ctx.collect_mux(tcid, wires::IMUX_MGT_REFCLK_PRE[1].cell(16));
    ctx.collect_mux(tcid, wires::IMUX_MGT_GREFCLK_PRE[0].cell(16));
    ctx.collect_mux(tcid, wires::IMUX_MGT_GREFCLK_PRE[1].cell(16));

    for i in 0..4 {
        ctx.collect_mux_ocd(tcid, wires::MGT_FWDCLK_S[i].cell(16), OcdMode::BitOrder);
        ctx.collect_mux_ocd(tcid, wires::MGT_FWDCLK_N[i].cell(16), OcdMode::BitOrder);
    }

    {
        let tcid = tcls::HCLK_MGT;
        for i in 0..8 {
            ctx.collect_progbuf(
                tcid,
                wires::HCLK_MGT[i].cell(0),
                wires::HCLK_ROW[i].cell(0).pos(),
            );
        }
        for i in 0..2 {
            ctx.collect_progbuf(
                tcid,
                wires::MGT_ROW[i].cell(0),
                wires::MGT_CLK_OUT[i].cell(0).pos(),
            );
        }
    }

    if !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
        let tcid = tcls::HCLK_MGT_BUF;
        for i in 0..2 {
            let wire = wires::MGT_ROW[i].cell(0);
            let diff = ctx.get_diff_routing_special(tcid, wire, specials::MGT_BUF_MGT);
            ctx.insert_support(tcid, BTreeSet::from_iter([wire]), xlat_bit_wide(diff));
        }
    }
}
