use prjcombine_entity::EntityId;
use prjcombine_interconnect::{db::BelAttributeType, grid::TileCoord};
use prjcombine_re_collector::diff::{
    DiffKey, SpecialId, xlat_bit, xlat_bit_bi, xlat_bit_wide, xlat_bitvec, xlat_enum_attr,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_spartan6::defs::{bcls, bslots, enums, tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
    spartan6::specials,
};

#[derive(Copy, Clone, Debug)]
struct ExtraBramFixup(SpecialId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraBramFixup {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let DiffKey::BelSpecial(tcid, bslot, _) = fuzzer.info.features[0].key else {
            unreachable!()
        };
        let key = DiffKey::BelSpecial(tcid, bslot, self.0);
        let rects = fuzzer.info.features[0]
            .rects
            .values()
            .take(4)
            .map(|&rect| rect.to_fixup())
            .collect();
        fuzzer.info.features.push(FuzzerFeature { key, rects });
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::BRAM);
    let mode16 = "RAMB16BWER";
    {
        let mut bctx = ctx.bel(bslots::BRAM[0]).sub(1);
        let mode = mode16;

        bctx.build()
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", "FULL")
            .test_bel_special(specials::BRAM_RAMB16)
            .mode(mode)
            .commit();
        for (attr, pname) in [
            (bcls::BRAM::CLKA_INV, "CLKA"),
            (bcls::BRAM::CLKB_INV, "CLKB"),
            (bcls::BRAM::ENA_INV, "ENA"),
            (bcls::BRAM::ENB_INV, "ENB"),
            (bcls::BRAM::RSTA_INV, "RSTA"),
            (bcls::BRAM::RSTB_INV, "RSTB"),
            (bcls::BRAM::REGCEA_INV, "REGCEA"),
            (bcls::BRAM::REGCEB_INV, "REGCEB"),
        ] {
            for (val, vname) in [(false, pname.to_string()), (true, format!("{pname}_B"))] {
                bctx.build()
                    .mode(mode)
                    .global_mutex("BRAM", "MULTI")
                    .tile_mutex("MODE", "FULL")
                    .pin(pname)
                    .test_bel_attr_special_bits_bi(attr, specials::BRAM_RAMB16, 0, val)
                    .attr(format!("{pname}INV"), vname)
                    .commit();
            }
        }
        bctx.mode(mode)
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", "FULL")
            .test_bel_attr_special_auto(bcls::BRAM::RAM_MODE, specials::BRAM_RAMB16);
        for (val, vname) in &backend.edev.db[enums::BRAM_RSTTYPE].values {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_bel_attr_special_val(bcls::BRAM::RSTTYPE_A, specials::BRAM_RAMB16, val)
                .attr("RSTTYPE", vname)
                .commit();
        }
        for attr in [bcls::BRAM::WRITE_MODE_A, bcls::BRAM::WRITE_MODE_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_bel_attr_special_auto(attr, specials::BRAM_RAMB16);
        }
        for attr in [bcls::BRAM::DATA_WIDTH_A, bcls::BRAM::DATA_WIDTH_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_bel_attr_special_auto(attr, specials::BRAM_RAMB16);
        }
        for attr in [bcls::BRAM::DOA_REG, bcls::BRAM::DOB_REG] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_bel_attr_bool_special_auto(attr, specials::BRAM_RAMB16, "0", "1");
        }
        for attr in [bcls::BRAM::RST_PRIORITY_A, bcls::BRAM::RST_PRIORITY_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_bel_attr_special_auto(attr, specials::BRAM_RAMB16);
        }
        for attr in [bcls::BRAM::EN_RSTRAM_A, bcls::BRAM::EN_RSTRAM_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .test_bel_attr_bool_special_auto(attr, specials::BRAM_RAMB16, "FALSE", "TRUE");
        }
        for attr in [
            bcls::BRAM::INIT_A,
            bcls::BRAM::INIT_B,
            bcls::BRAM::SRVAL_A,
            bcls::BRAM::SRVAL_B,
        ] {
            let aname = backend.edev.db[bcls::BRAM].attributes.key(attr);
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "36")
                .attr("DATA_WIDTH_B", "36")
                .test_bel_attr_special_bits(attr, specials::BRAM_RAMB16, 0)
                .multi_attr(aname, MultiValue::Hex(0), 36)
        }
        for i in 0..0x40 {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "18")
                .test_bel_attr_special_bits(
                    bcls::BRAM::DATA,
                    specials::BRAM_RAMB16_NARROW,
                    i * 0x100,
                )
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "36")
                .test_bel_attr_special_bits(bcls::BRAM::DATA, specials::BRAM_RAMB16_WIDE, i * 0x100)
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for i in 0..8 {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "18")
                .test_bel_attr_special_bits(
                    bcls::BRAM::DATAP,
                    specials::BRAM_RAMB16_NARROW,
                    i * 0x100,
                )
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "36")
                .test_bel_attr_special_bits(
                    bcls::BRAM::DATAP,
                    specials::BRAM_RAMB16_WIDE,
                    i * 0x100,
                )
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
        }

        for (attr, opt, val) in [
            (bcls::BRAM::EN_WEAK_WRITE_A, "ENWEAKWRITEA", "ENABLE"),
            (bcls::BRAM::EN_WEAK_WRITE_B, "ENWEAKWRITEB", "ENABLE"),
            (bcls::BRAM::WEAK_WRITE_VAL_A, "WEAKWRITEVALA", "1"),
            (bcls::BRAM::WEAK_WRITE_VAL_B, "WEAKWRITEVALB", "1"),
        ] {
            bctx.mode(mode)
                .global_mutex_here("BRAM")
                .tile_mutex("MODE", "FULL")
                .test_bel_attr_special(attr, specials::BRAM_RAMB16)
                .global(opt, val)
                .commit();
        }
    }
    for idx in 0..2 {
        let bslot = bslots::BRAM[idx];
        let mut bctx = ctx.bel(bslot);
        let mode = "RAMB8BWER";
        let bel_name = backend.edev.db.bel_slots.key(bslot).as_str();
        bctx.build()
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", bel_name)
            .prop(ExtraBramFixup(specials::BRAM_RAMB8_FIXUP))
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for (attr, pname) in [
            (bcls::BRAM::CLKA_INV, "CLKAWRCLK"),
            (bcls::BRAM::CLKB_INV, "CLKBRDCLK"),
            (bcls::BRAM::ENA_INV, "ENAWREN"),
            (bcls::BRAM::ENB_INV, "ENBRDEN"),
            (bcls::BRAM::RSTA_INV, "RSTA"),
            (bcls::BRAM::RSTB_INV, "RSTBRST"),
            (bcls::BRAM::REGCEA_INV, "REGCEA"),
            (bcls::BRAM::REGCEB_INV, "REGCEBREGCE"),
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .pin(pname)
                .test_bel_attr_bool_rename(
                    format!("{pname}INV"),
                    attr,
                    pname,
                    format!("{pname}_B"),
                );
        }
        for (pins, ab, ul) in [(bcls::BRAM::WEA, 'A', 'L'), (bcls::BRAM::WEB, 'B', 'U')] {
            for j in 0..2 {
                let pname = format!("WE{ab}WE{ul}{j}");
                for (val, vname) in [(false, pname.to_string()), (true, format!("{pname}_B"))] {
                    bctx.mode(mode)
                        .global_mutex("BRAM", "MULTI")
                        .tile_mutex("MODE", "HALF")
                        .pin(&pname)
                        .test_bel_input_inv(pins[j], val)
                        .attr(format!("{pname}INV"), vname)
                        .commit();
                }
                let pname = format!("WE{ab}{ii}", ii = idx * 2 + j);
                for (val, vname) in [(false, pname.to_string()), (true, format!("{pname}_B"))] {
                    bctx.build()
                        .bel_sub_mode(bslots::BRAM[0], 1, mode16)
                        .global_mutex("BRAM", "MULTI")
                        .tile_mutex("MODE", "FULL")
                        .bel_sub_pin(bslots::BRAM[0], 1, &pname)
                        .test_bel_input_inv(pins[j], val)
                        .bel_sub_attr(bslots::BRAM[0], 1, format!("{pname}INV"), vname)
                        .commit();
                }
            }
        }
        bctx.mode(mode)
            .global_mutex("BRAM", "MULTI")
            .tile_mutex("MODE", "HALF")
            .test_bel_attr_auto(bcls::BRAM::RAM_MODE);
        for (val, vname) in &backend.edev.db[enums::BRAM_RSTTYPE].values {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_val(bcls::BRAM::RSTTYPE_A, val)
                .attr("RSTTYPE", vname)
                .commit();
        }
        for attr in [bcls::BRAM::WRITE_MODE_A, bcls::BRAM::WRITE_MODE_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_auto(attr);
        }
        for attr in [bcls::BRAM::DATA_WIDTH_A, bcls::BRAM::DATA_WIDTH_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_auto(attr);
        }
        for attr in [bcls::BRAM::DOA_REG, bcls::BRAM::DOB_REG] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_bool_auto(attr, "0", "1");
        }
        for attr in [bcls::BRAM::RST_PRIORITY_A, bcls::BRAM::RST_PRIORITY_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_auto(attr);
        }
        for attr in [bcls::BRAM::EN_RSTRAM_A, bcls::BRAM::EN_RSTRAM_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        for attr in [
            bcls::BRAM::INIT_A,
            bcls::BRAM::INIT_B,
            bcls::BRAM::SRVAL_A,
            bcls::BRAM::SRVAL_B,
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .attr("DATA_WIDTH_A", "18")
                .attr("DATA_WIDTH_B", "18")
                .test_bel_attr_multi(attr, MultiValue::Hex(0));
        }
        for i in 0..0x20 {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_bits_base(bcls::BRAM::DATA, i * 0x100)
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for i in 0..4 {
            bctx.mode(mode)
                .global_mutex("BRAM", "MULTI")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_bits_base(bcls::BRAM::DATAP, i * 0x100)
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for (attr, opt, val) in [
            (bcls::BRAM::EN_WEAK_WRITE_A, "ENWEAKWRITEA", "ENABLE"),
            (bcls::BRAM::EN_WEAK_WRITE_B, "ENWEAKWRITEB", "ENABLE"),
            (bcls::BRAM::WEAK_WRITE_VAL_A, "WEAKWRITEVALA", "1"),
            (bcls::BRAM::WEAK_WRITE_VAL_B, "WEAKWRITEVALB", "1"),
        ] {
            bctx.mode(mode)
                .global_mutex_here("BRAM")
                .tile_mutex("MODE", "HALF")
                .test_bel_attr_bits(attr)
                .global(opt, val)
                .commit();
        }

        for (attr, opt) in [
            (bcls::BRAM::DDEL_A, ["BRAM_DDEL_A_D", "BRAM_DDEL_A_U"][idx]),
            (bcls::BRAM::DDEL_B, ["BRAM_DDEL_B_D", "BRAM_DDEL_B_U"][idx]),
        ] {
            for (val, vname) in [(0, "0"), (1, "1"), (3, "11"), (7, "111")] {
                bctx.build()
                    .bel_sub_mode(bslots::BRAM[0], 1, mode16)
                    .global_mutex_here("BRAM")
                    .tile_mutex("MODE", "FULL")
                    .test_bel_attr_bitvec_u32(attr, val)
                    .global(opt, vname)
                    .commit();
            }
        }
        for (attr, opt) in [
            (bcls::BRAM::WDEL_A, ["BRAM_WDEL_A_D", "BRAM_WDEL_A_U"][idx]),
            (bcls::BRAM::WDEL_B, ["BRAM_WDEL_B_D", "BRAM_WDEL_B_U"][idx]),
        ] {
            for (val, vname) in ["0", "1", "10", "11", "100", "101", "110", "111"]
                .into_iter()
                .enumerate()
            {
                bctx.build()
                    .bel_sub_mode(bslots::BRAM[0], 1, mode16)
                    .global_mutex_here("BRAM")
                    .tile_mutex("MODE", "FULL")
                    .test_bel_attr_bitvec_u32(attr, val as u32)
                    .global(opt, vname)
                    .commit();
            }
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    for (bslot, attr, opt) in [
        (bslots::BRAM[0], bcls::BRAM::BW_EN_A, "BW_EN_A_D"),
        (bslots::BRAM[1], bcls::BRAM::BW_EN_A, "BW_EN_A_U"),
        (bslots::BRAM[0], bcls::BRAM::BW_EN_B, "BW_EN_B_D"),
        (bslots::BRAM[1], bcls::BRAM::BW_EN_B, "BW_EN_B_U"),
    ] {
        ctx.build()
            .global_mutex("BRAM", "NONE")
            .extra_tiles_by_bel_attr_bits(bslot, attr)
            .test_global_special(specials::BRAM_BW_EN)
            .global(opt, "1")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::BRAM;

    for bslot in bslots::BRAM {
        for pin in [
            bcls::BRAM::WEA[0],
            bcls::BRAM::WEA[1],
            bcls::BRAM::WEB[0],
            bcls::BRAM::WEB[1],
        ] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        for attr in [bcls::BRAM::DDEL_A, bcls::BRAM::DDEL_B] {
            ctx.collect_bel_attr_sparse(tcid, bslot, attr, [0, 1, 3, 7]);
        }
        for attr in [bcls::BRAM::WDEL_A, bcls::BRAM::WDEL_B] {
            ctx.collect_bel_attr_sparse(tcid, bslot, attr, 0..8);
        }

        ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::BW_EN_A);
        ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::BW_EN_B);
    }

    for attr in [
        bcls::BRAM::CLKA_INV,
        bcls::BRAM::CLKB_INV,
        bcls::BRAM::ENA_INV,
        bcls::BRAM::ENB_INV,
        bcls::BRAM::RSTA_INV,
        bcls::BRAM::RSTB_INV,
        bcls::BRAM::REGCEA_INV,
        bcls::BRAM::REGCEB_INV,
        bcls::BRAM::DOA_REG,
        bcls::BRAM::DOB_REG,
        bcls::BRAM::EN_RSTRAM_A,
        bcls::BRAM::EN_RSTRAM_B,
    ] {
        let diff0_f = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslots::BRAM[0],
            attr,
            specials::BRAM_RAMB16,
            0,
            false,
        );
        let diff0_h0 = ctx.get_diff_attr_bool_bi(tcid, bslots::BRAM[0], attr, false);
        let diff0_h1 = ctx.get_diff_attr_bool_bi(tcid, bslots::BRAM[1], attr, false);
        assert_eq!(diff0_f, diff0_h0.combine(&diff0_h1));
        let diff1_f = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslots::BRAM[0],
            attr,
            specials::BRAM_RAMB16,
            0,
            true,
        );
        let diff1_h0 = ctx.get_diff_attr_bool_bi(tcid, bslots::BRAM[0], attr, true);
        let diff1_h1 = ctx.get_diff_attr_bool_bi(tcid, bslots::BRAM[1], attr, true);
        assert_eq!(diff1_f, diff1_h0.combine(&diff1_h1));
        ctx.insert_bel_attr_bool(tcid, bslots::BRAM[0], attr, xlat_bit_bi(diff0_h0, diff1_h0));
        ctx.insert_bel_attr_bool(tcid, bslots::BRAM[1], attr, xlat_bit_bi(diff0_h1, diff1_h1));
    }

    for attr in [
        bcls::BRAM::RAM_MODE,
        bcls::BRAM::WRITE_MODE_A,
        bcls::BRAM::WRITE_MODE_B,
        bcls::BRAM::RST_PRIORITY_A,
        bcls::BRAM::RST_PRIORITY_B,
        bcls::BRAM::DATA_WIDTH_A,
        bcls::BRAM::DATA_WIDTH_B,
    ] {
        let mut diffs_h0 = vec![];
        let mut diffs_h1 = vec![];
        let BelAttributeType::Enum(ecid) = ctx.edev.db[bcls::BRAM].attributes[attr].typ else {
            unreachable!()
        };
        for val in ctx.edev.db[ecid].values.ids() {
            let diff_f = ctx.get_diff_attr_special_val(
                tcid,
                bslots::BRAM[0],
                attr,
                specials::BRAM_RAMB16,
                val,
            );
            let diff_h0 = ctx.get_diff_attr_val(tcid, bslots::BRAM[0], attr, val);
            let diff_h1 = ctx.get_diff_attr_val(tcid, bslots::BRAM[1], attr, val);
            assert_eq!(diff_f, diff_h0.combine(&diff_h1));
            diffs_h0.push((val, diff_h0));
            diffs_h1.push((val, diff_h1));
        }
        ctx.insert_bel_attr_enum(tcid, bslots::BRAM[0], attr, xlat_enum_attr(diffs_h0));
        ctx.insert_bel_attr_enum(tcid, bslots::BRAM[1], attr, xlat_enum_attr(diffs_h1));
    }
    {
        let mut diffs_h0_a = vec![];
        let mut diffs_h0_b = vec![];
        let mut diffs_h1_a = vec![];
        let mut diffs_h1_b = vec![];
        for val in ctx.edev.db[enums::BRAM_RSTTYPE].values.ids() {
            let diff_f = ctx.get_diff_attr_special_val(
                tcid,
                bslots::BRAM[0],
                bcls::BRAM::RSTTYPE_A,
                specials::BRAM_RAMB16,
                val,
            );
            let mut diff_h0 =
                ctx.get_diff_attr_val(tcid, bslots::BRAM[0], bcls::BRAM::RSTTYPE_A, val);
            let mut diff_h1 =
                ctx.get_diff_attr_val(tcid, bslots::BRAM[1], bcls::BRAM::RSTTYPE_A, val);
            assert_eq!(diff_f, diff_h0.combine(&diff_h1));
            let diff_h0_b = diff_h0.split_bits_by(|bit| bit.bit.to_idx() == 15);
            let diff_h1_b = diff_h1.split_bits_by(|bit| bit.bit.to_idx() == 48);
            diffs_h0_a.push((val, diff_h0));
            diffs_h0_b.push((val, diff_h0_b));
            diffs_h1_a.push((val, diff_h1));
            diffs_h1_b.push((val, diff_h1_b));
        }
        ctx.insert_bel_attr_enum(
            tcid,
            bslots::BRAM[0],
            bcls::BRAM::RSTTYPE_A,
            xlat_enum_attr(diffs_h0_a),
        );
        ctx.insert_bel_attr_enum(
            tcid,
            bslots::BRAM[0],
            bcls::BRAM::RSTTYPE_B,
            xlat_enum_attr(diffs_h0_b),
        );
        ctx.insert_bel_attr_enum(
            tcid,
            bslots::BRAM[1],
            bcls::BRAM::RSTTYPE_A,
            xlat_enum_attr(diffs_h1_a),
        );
        ctx.insert_bel_attr_enum(
            tcid,
            bslots::BRAM[1],
            bcls::BRAM::RSTTYPE_B,
            xlat_enum_attr(diffs_h1_b),
        );
    }
    for attr in [
        bcls::BRAM::EN_WEAK_WRITE_A,
        bcls::BRAM::EN_WEAK_WRITE_B,
        bcls::BRAM::WEAK_WRITE_VAL_A,
        bcls::BRAM::WEAK_WRITE_VAL_B,
    ] {
        let diff1_f =
            ctx.get_diff_bel_attr_special(tcid, bslots::BRAM[0], attr, specials::BRAM_RAMB16);
        let diff1_h0 = ctx.get_diff_attr_bool(tcid, bslots::BRAM[0], attr);
        let diff1_h1 = ctx.get_diff_attr_bool(tcid, bslots::BRAM[1], attr);
        assert_eq!(diff1_f, diff1_h0.combine(&diff1_h1));
        ctx.insert_bel_attr_bool(tcid, bslots::BRAM[0], attr, xlat_bit(diff1_h0));
        ctx.insert_bel_attr_bool(tcid, bslots::BRAM[1], attr, xlat_bit(diff1_h1));
    }

    let mut present_f = ctx.get_diff_bel_special(tcid, bslots::BRAM[0], specials::BRAM_RAMB16);
    let mut present_h0 = ctx.get_diff_bel_special(tcid, bslots::BRAM[0], specials::PRESENT);
    let mut present_h1 = ctx.get_diff_bel_special(tcid, bslots::BRAM[1], specials::PRESENT);

    for pin in [
        bcls::BRAM::WEA[0],
        bcls::BRAM::WEA[1],
        bcls::BRAM::WEB[0],
        bcls::BRAM::WEB[1],
    ] {
        let bit = ctx.bel_input_inv(tcid, bslots::BRAM[0], pin);
        present_f.apply_bit_diff(bit, false, true);
        present_h0.apply_bit_diff(bit, false, true);
        let bit = ctx.bel_input_inv(tcid, bslots::BRAM[1], pin);
        present_f.apply_bit_diff(bit, false, true);
        present_h1.apply_bit_diff(bit, false, true);
    }
    for attr in [
        bcls::BRAM::REGCEA_INV,
        bcls::BRAM::REGCEB_INV,
        bcls::BRAM::RSTB_INV,
        bcls::BRAM::ENA_INV,
        bcls::BRAM::ENB_INV,
    ] {
        let bit = ctx.bel_attr_bit(tcid, bslots::BRAM[0], attr);
        present_f.apply_bit_diff(bit, false, true);
        present_h0.apply_bit_diff(bit, false, true);
        let bit = ctx.bel_attr_bit(tcid, bslots::BRAM[1], attr);
        present_f.apply_bit_diff(bit, false, true);
        present_h1.apply_bit_diff(bit, false, true);
    }

    for attr in [bcls::BRAM::BW_EN_A, bcls::BRAM::BW_EN_B] {
        let bit = ctx.bel_attr_bit(tcid, bslots::BRAM[0], attr);
        present_f.apply_bit_diff(bit, true, false);
        present_h0.apply_bit_diff(bit, true, false);
        let bit = ctx.bel_attr_bit(tcid, bslots::BRAM[1], attr);
        present_f.apply_bit_diff(bit, true, false);
        present_h1.apply_bit_diff(bit, true, false);
    }
    for attr in [bcls::BRAM::DDEL_A, bcls::BRAM::DDEL_B] {
        let val = if ctx.device.name.ends_with("l") { 7 } else { 3 };
        let item = ctx.bel_attr_bitvec(tcid, bslots::BRAM[0], attr);
        present_f.apply_bitvec_diff_int(item, val, 0);
        present_h0.apply_bitvec_diff_int(item, val, 0);
        let item = ctx.bel_attr_bitvec(tcid, bslots::BRAM[1], attr);
        present_f.apply_bitvec_diff_int(item, val, 0);
        present_h1.apply_bitvec_diff_int(item, val, 0);
    }

    assert_eq!(present_h0, present_h1);
    assert_eq!(present_h0, present_f);
    let present_h0_fixup =
        ctx.get_diff_bel_special(tcid, bslots::BRAM[0], specials::BRAM_RAMB8_FIXUP);
    let present_h1_fixup =
        ctx.get_diff_bel_special(tcid, bslots::BRAM[1], specials::BRAM_RAMB8_FIXUP);
    assert_eq!(present_h0_fixup, present_h1_fixup);
    assert_eq!(present_h0_fixup, present_f);
    let bits = xlat_bit_wide(present_f);
    assert_eq!(bits.len(), 2);
    for (bel, bit) in [(bslots::BRAM[0], bits[0]), (bslots::BRAM[1], bits[1])] {
        ctx.insert_bel_attr_bool(tcid, bel, bcls::BRAM::COMBINE, bit);
    }

    for attr in [
        bcls::BRAM::SRVAL_A,
        bcls::BRAM::SRVAL_B,
        bcls::BRAM::INIT_A,
        bcls::BRAM::INIT_B,
    ] {
        let diffs_f =
            ctx.get_diffs_attr_special_bits(tcid, bslots::BRAM[0], attr, specials::BRAM_RAMB16, 36);
        let diffs_h0 = ctx.get_diffs_attr_bits(tcid, bslots::BRAM[0], attr, 18);
        let diffs_h1 = ctx.get_diffs_attr_bits(tcid, bslots::BRAM[1], attr, 18);
        assert_eq!(diffs_f[0..16], diffs_h0[0..16]);
        assert_eq!(diffs_f[16..32], diffs_h1[0..16]);
        assert_eq!(diffs_f[32..34], diffs_h0[16..18]);
        assert_eq!(diffs_f[34..36], diffs_h1[16..18]);
        ctx.insert_bel_attr_bitvec(tcid, bslots::BRAM[0], attr, xlat_bitvec(diffs_h0));
        ctx.insert_bel_attr_bitvec(tcid, bslots::BRAM[1], attr, xlat_bitvec(diffs_h1));
    }

    let diffs_n = ctx.get_diffs_attr_special_bits(
        tcid,
        bslots::BRAM[0],
        bcls::BRAM::DATA,
        specials::BRAM_RAMB16_NARROW,
        0x4000,
    );
    let diffs_w = ctx.get_diffs_attr_special_bits(
        tcid,
        bslots::BRAM[0],
        bcls::BRAM::DATA,
        specials::BRAM_RAMB16_WIDE,
        0x4000,
    );
    let diffs_h0 = ctx.get_diffs_attr_bits(tcid, bslots::BRAM[0], bcls::BRAM::DATA, 0x2000);
    let diffs_h1 = ctx.get_diffs_attr_bits(tcid, bslots::BRAM[1], bcls::BRAM::DATA, 0x2000);
    assert_eq!(diffs_n[..0x2000], diffs_h0);
    assert_eq!(diffs_n[0x2000..], diffs_h1);
    for i in 0..4000 {
        let iw = i & 0xf | i >> 9 & 0x10 | i << 1 & 0x3fe0;
        assert_eq!(diffs_n[i], diffs_w[iw]);
    }
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslots::BRAM[0],
        bcls::BRAM::DATA,
        xlat_bitvec(diffs_h0),
    );
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslots::BRAM[1],
        bcls::BRAM::DATA,
        xlat_bitvec(diffs_h1),
    );

    let diffs_n = ctx.get_diffs_attr_special_bits(
        tcid,
        bslots::BRAM[0],
        bcls::BRAM::DATAP,
        specials::BRAM_RAMB16_NARROW,
        0x800,
    );
    let diffs_w = ctx.get_diffs_attr_special_bits(
        tcid,
        bslots::BRAM[0],
        bcls::BRAM::DATAP,
        specials::BRAM_RAMB16_WIDE,
        0x800,
    );
    let diffs_h0 = ctx.get_diffs_attr_bits(tcid, bslots::BRAM[0], bcls::BRAM::DATAP, 0x400);
    let diffs_h1 = ctx.get_diffs_attr_bits(tcid, bslots::BRAM[1], bcls::BRAM::DATAP, 0x400);
    assert_eq!(diffs_n[..0x400], diffs_h0);
    assert_eq!(diffs_n[0x400..], diffs_h1);
    for i in 0..800 {
        let iw = i & 1 | i >> 9 & 2 | i << 1 & 0x7fc;
        assert_eq!(diffs_n[i], diffs_w[iw]);
    }
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslots::BRAM[0],
        bcls::BRAM::DATAP,
        xlat_bitvec(diffs_h0),
    );
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslots::BRAM[1],
        bcls::BRAM::DATAP,
        xlat_bitvec(diffs_h1),
    );
}
