use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::TileItem;
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{
        xlat_bit, xlat_bit_wide, xlat_bitvec, xlat_enum, xlat_enum_ocd, CollectorCtx, Diff, OcdMode,
    },
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_multi_attr_dec, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for tile in ["IOI.LR", "IOI.BT"] {
        let node_iob = backend.egrid.db.get_node("IOB");
        for i in 0..2 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("ILOGIC{i}"),
                TileBits::MainAuto,
            );
            let bel_other = BelId::from_idx(1 - i);
            let bel_ologic = BelId::from_idx(2 + i);
            let bel_ioiclk = BelId::from_idx(7 + i);
            for mode in ["ILOGIC2", "ISERDES2"] {
                fuzz_one!(ctx, "MODE", mode, [
                    (tile_mutex "CLK", "TEST_LOGIC"),
                    (global_opt "GLUTMASK", "NO"),
                    (bel_unused bel_other),
                    (related TileRelation::Delta(0, 0, node_iob), (nop))
                ], [
                    (mode mode)
                ]);
            }
            fuzz_enum!(ctx, "IFFTYPE", ["#LATCH", "#FF", "DDR"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (tile_mutex "CLK", "NOPE")
            ]);
            fuzz_enum!(ctx, "D2OBYP_SEL", ["GND", "T"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (attr "FABRICOUTUSED", "0"),
                (pin "TFB"),
                (pin "FABRICOUT")
            ]);
            fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (bel_unused bel_other),
                (tile_mutex "CLK", "NOPE"),
                (attr "FABRICOUTUSED", "0"),
                (attr "IFFTYPE", "#FF"),
                (attr "D2OBYP_SEL", "GND"),
                (pin "OFB"),
                (pin "D"),
                (pin "DDLY")
            ]);
            fuzz_enum!(ctx, "IFFMUX", ["0", "1"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (bel_unused bel_other),
                (tile_mutex "CLK", "NOPE"),
                (attr "FABRICOUTUSED", "0"),
                (attr "IFFTYPE", "#FF"),
                (attr "D2OBYP_SEL", "GND"),
                (pin "OFB"),
                (pin "D"),
                (pin "DDLY")
            ]);
            fuzz_enum!(ctx, "SRINIT_Q", ["0", "1"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2")
            ]);
            fuzz_enum!(ctx, "SRTYPE_Q", ["ASYNC", "SYNC"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2")
            ]);
            fuzz_enum!(ctx, "SRUSED", ["0"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (pin "SR"),
                (attr "IFFTYPE", "#FF")
            ]);
            fuzz_enum!(ctx, "REVUSED", ["0"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (pin "REV"),
                (attr "IFFTYPE", "#FF")
            ]);
            fuzz_one!(ctx, "IFF_CE_ENABLE", "0", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (pin "CE0"),
                (attr "IFFTYPE", "#FF")
            ], [
                (pin_pips "CE0")
            ]);

            fuzz_enum!(ctx, "DATA_WIDTH", ["1", "2", "3", "4", "5", "6", "7", "8"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ISERDES2")
            ]);
            fuzz_enum!(ctx, "BITSLIP_ENABLE", ["FALSE", "TRUE"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ISERDES2")
            ]);
            fuzz_enum!(ctx, "INTERFACE_TYPE", ["NETWORKING", "NETWORKING_PIPELINED", "RETIMED"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ISERDES2")
            ]);
            fuzz_one!(ctx, "MUX.SR", "INT", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (mutex "MUX.SR", "INT")
            ], [
                (pip (pin "SR_INT"), (pin "SR"))
            ]);
            fuzz_one!(ctx, "MUX.SR", "OLOGIC_SR", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "ILOGIC2"),
                (mutex "MUX.SR", "OLOGIC_SR")
            ], [
                (pip (bel_pin_far bel_ologic, "SR"), (pin "SR"))
            ]);

            fuzz_one!(ctx, "MUX.CLK", format!("ICLK{i}"), [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_LOGIC")
            ], [
                (pip (bel_pin bel_ioiclk, "CLK0_ILOGIC"), (pin "CLK0"))
            ]);
            fuzz_one!(ctx, "ENABLE.IOCE", "1", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_LOGIC"),
                (mode "ISERDES2"),
                (bel_mode bel_other, "ISERDES2"),
                (pin "D"),
                (bel_pin bel_other, "D")
            ], [
                (pip (bel_pin bel_ioiclk, "IOCE0"), (pin "IOCE"))
            ]);
            fuzz_one!(ctx, "ENABLE", "1", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_LOGIC"),
                (unused),
                (bel_unused bel_other)
            ], [
                (pip (bel_pin bel_ioiclk, "IOCE0"), (pin "IOCE"))
            ]);

            if i == 1 {
                fuzz_one!(ctx, "MUX.D", "OTHER_IOB_I", [
                    (related TileRelation::Delta(0, 0, node_iob), (nop))
                ], [
                    (pip (bel_pin bel_other, "IOB_I"), (pin "D_MUX"))
                ]);
            }
        }
        for i in 0..2 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("OLOGIC{i}"),
                TileBits::MainAuto,
            );
            let bel_iodelay = BelId::from_idx(4 + i);
            let bel_ioiclk = BelId::from_idx(7 + i);
            let bel_ioi = BelId::from_idx(9);
            for mode in ["OLOGIC2", "OSERDES2"] {
                fuzz_one!(ctx, "MODE", mode, [
                    (related TileRelation::Delta(0, 0, node_iob), (nop)),
                    (global_opt "ENABLEMISR", "N"),
                    (tile_mutex "CLK", "TEST_LOGIC"),
                    (global_opt "GLUTMASK", "NO")
                ], [
                    (mode mode)
                ]);
            }
            fuzz_one!(ctx, "MODE", "OLOGIC2.MISR_RESET", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (global_opt "ENABLEMISR", "Y"),
                (global_opt "MISRRESET", "Y"),
                (tile_mutex "CLK", "TEST_LOGIC"),
                (global_opt "GLUTMASK", "NO")
            ], [
                (mode "OLOGIC2")
            ]);

            fuzz_enum!(ctx, "SRINIT_OQ", ["0", "1"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2")
            ]);
            fuzz_enum!(ctx, "SRINIT_TQ", ["0", "1"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2")
            ]);
            fuzz_enum!(ctx, "SRTYPE_OQ", ["SYNC", "ASYNC"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "SRTYPE_TQ", ["SYNC", "ASYNC"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "DATA_WIDTH", ["1", "2", "3", "4", "5", "6", "7", "8"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OSERDES2")
            ]);
            fuzz_enum!(ctx, "BYPASS_GCLK_FF", ["FALSE", "TRUE"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OSERDES2")
            ]);
            fuzz_enum!(ctx, "OUTPUT_MODE", ["DIFFERENTIAL", "SINGLE_ENDED"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OSERDES2")
            ]);
            for attr in ["OSRUSED", "TSRUSED", "OREVUSED", "TREVUSED"] {
                fuzz_enum!(ctx, attr, ["0"], [
                    (related TileRelation::Delta(0, 0, node_iob), (nop)),
                    (mode "OLOGIC2"),
                    (attr "OUTFFTYPE", "#FF"),
                    (attr "TFFTYPE", "#FF"),
                    (pin "SR"),
                    (pin "REV")
                ]);
            }
            fuzz_one!(ctx, "MUX.SR", "INT", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mutex "MUX.SR", "INT"),
                (mode "OLOGIC2")
            ], [
                (pin_pips "SR")
            ]);
            fuzz_one!(ctx, "MUX.REV", "INT", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mutex "MUX.REV", "INT"),
                (mode "OLOGIC2")
            ], [
                (pin_pips "REV")
            ]);
            fuzz_one!(ctx, "MUX.OCE", "INT", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mutex "MUX.OCE", "INT"),
                (attr "OUTFFTYPE", "#FF"),
                (mode "OLOGIC2")
            ], [
                (pin_pips "OCE")
            ]);
            fuzz_one!(ctx, "MUX.OCE", "PCI_CE", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mutex "MUX.OCE", "PCI_CE"),
                (attr "OUTFFTYPE", "#FF"),
                (mode "OLOGIC2")
            ], [
                (pip (bel_pin bel_ioi, "PCI_CE"), (pin "OCE"))
            ]);
            fuzz_one!(ctx, "MUX.TCE", "INT", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mutex "MUX.TCE", "INT"),
                (attr "TFFTYPE", "#FF"),
                (mode "OLOGIC2")
            ], [
                (pin_pips "TCE")
            ]);
            fuzz_one!(ctx, "MUX.TRAIN", "MCB", [
                (global_mutex "DRPSDO", "NOPE"),
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mutex "MUX.TRAIN", "MCB"),
                (mode "OSERDES2")
            ], [
                (pip (bel_pin bel_ioi, "MCB_DRPTRAIN"), (pin "TRAIN"))
            ]);
            fuzz_one!(ctx, "MUX.TRAIN", "INT", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mutex "MUX.TRAIN", "INT"),
                (mode "OSERDES2")
            ], [
                (pin_pips "TRAIN")
            ]);
            fuzz_multi_attr_dec!(ctx, "TRAIN_PATTERN", 4, [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OSERDES2")
            ]);
            fuzz_one!(ctx, "MUX.D", "MCB", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OSERDES2"),
                (global_mutex "DRPSDO", "USE"),
                (pip (bel_pin bel_ioi, "MCB_DRPSDO"), (bel_pin bel_iodelay, "CE"))
            ], [
                (pip (pin "MCB_D1"), (pin "D1"))
            ]);
            fuzz_enum!(ctx, "OUTFFTYPE", ["#LATCH", "#FF", "DDR"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2"),
                (tile_mutex "CLK", "NOPE"),
                (attr "TFFTYPE", "")
            ]);
            fuzz_enum!(ctx, "TFFTYPE", ["#LATCH", "#FF", "DDR"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2"),
                (tile_mutex "CLK", "NOPE"),
                (attr "OUTFFTYPE", "")
            ]);
            fuzz_enum!(ctx, "OMUX", ["D1", "OUTFF"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2"),
                (tile_mutex "CLK", "NOPE"),
                (attr "OUTFFTYPE", "#FF"),
                (attr "D1USED", "0"),
                (attr "O1USED", "0"),
                (pin "D1"),
                (pin "OQ")
            ]);
            fuzz_enum!(ctx, "OT1USED", ["0"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2"),
                (tile_mutex "CLK", "NOPE"),
                (attr "OUTFFTYPE", ""),
                (attr "TFFTYPE", ""),
                (attr "T1USED", "0"),
                (pin "T1"),
                (pin "TQ")
            ]);
            fuzz_enum!(ctx, "DDR_ALIGNMENT", ["NONE", "C0"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2"),
                (tile_mutex "CLK", "NOPE"),
                (attr "OUTFFTYPE", "DDR"),
                (attr "TDDR_ALIGNMENT", "")
            ]);
            fuzz_enum!(ctx, "TDDR_ALIGNMENT", ["NONE", "C0"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "OLOGIC2"),
                (tile_mutex "CLK", "NOPE"),
                (attr "TFFTYPE", "DDR"),
                (attr "DDR_ALIGNMENT", "")
            ]);
            fuzz_enum!(ctx, "MISRATTRBOX", ["MISR_ENABLE_DATA"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (global_opt "ENABLEMISR", "Y"),
                (global_opt "MISR_BLV_EN", "Y"),
                (global_opt "MISR_BLH_EN", "Y"),
                (global_opt "MISR_BRV_EN", "Y"),
                (global_opt "MISR_BRH_EN", "Y"),
                (global_opt "MISR_TLV_EN", "Y"),
                (global_opt "MISR_TLH_EN", "Y"),
                (global_opt "MISR_TRV_EN", "Y"),
                (global_opt "MISR_TRH_EN", "Y"),
                (global_opt "MISR_BM_EN", "Y"),
                (global_opt "MISR_TM_EN", "Y"),
                (mode "OLOGIC2")
            ]);
            fuzz_enum!(ctx, "MISR_ENABLE_CLK", ["CLK0", "CLK1"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (global_opt "ENABLEMISR", "Y"),
                (mode "OLOGIC2")
            ]);

            fuzz_one!(ctx, "MUX.CLK", format!("OCLK{i}"), [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_LOGIC")
            ], [
                (pip (bel_pin bel_ioiclk, "CLK0_OLOGIC"), (pin "CLK0"))
            ]);
            fuzz_one!(ctx, "ENABLE.IOCE", "1", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_LOGIC"),
                (mode "OSERDES2")
            ], [
                (pip (bel_pin bel_ioiclk, "IOCE1"), (pin "IOCE"))
            ]);
            fuzz_one!(ctx, "ENABLE", "1", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_LOGIC"),
                (unused)
            ], [
                (pip (bel_pin bel_ioiclk, "IOCE1"), (pin "IOCE"))
            ]);
        }
        for i in 0..2 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("IODELAY{i}"),
                TileBits::MainAuto,
            );
            let bel_other = BelId::from_idx(5 - i);
            let bel_ilogic = BelId::from_idx(i);
            let bel_ologic = BelId::from_idx(2 + i);
            let bel_ioiclk = BelId::from_idx(7 + i);
            for mode in ["IODELAY2", "IODRP2", "IODRP2_MCB"] {
                fuzz_one!(ctx, "MODE", mode, [
                    (related TileRelation::Delta(0, 0, node_iob), (nop)),
                    (global_mutex "DRPSDO", "NOPE"),
                    (global_opt "GLUTMASK", "NO"),
                    (global_opt "IOI_TESTPCOUNTER", "NO"),
                    (global_opt "IOI_TESTNCOUNTER", "NO"),
                    (global_opt "IOIENFFSCAN_DRP", "NO"),
                    (bel_unused bel_other)
                ], [
                    (mode mode)
                ]);
            }
            fuzz_one!(ctx, "MODE", "IODELAY2.TEST_PCOUNTER", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (global_opt "GLUTMASK", "NO"),
                (global_opt "IOI_TESTPCOUNTER", "YES"),
                (global_opt "IOI_TESTNCOUNTER", "NO"),
                (global_opt "IOIENFFSCAN_DRP", "NO"),
                (bel_unused bel_other)
            ], [
                (mode "IODELAY2")
            ]);
            fuzz_one!(ctx, "MODE", "IODELAY2.TEST_NCOUNTER", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (global_opt "GLUTMASK", "NO"),
                (global_opt "IOI_TESTPCOUNTER", "NO"),
                (global_opt "IOI_TESTNCOUNTER", "YES"),
                (global_opt "IOIENFFSCAN_DRP", "NO"),
                (bel_unused bel_other)
            ], [
                (mode "IODELAY2")
            ]);
            fuzz_one!(ctx, "MODE", "IODRP2.IOIENFFSCAN_DRP", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (global_opt "GLUTMASK", "NO"),
                (global_opt "IOI_TESTPCOUNTER", "NO"),
                (global_opt "IOI_TESTNCOUNTER", "NO"),
                (global_opt "IOIENFFSCAN_DRP", "YES"),
                (bel_unused bel_other)
            ], [
                (mode "IODRP2")
            ]);

            fuzz_multi_attr_dec!(ctx, "ODELAY_VALUE", 8, [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2")
            ]);
            fuzz_multi_attr_dec!(ctx, "IDELAY_VALUE", 8, [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2"),
                (attr "IDELAY_TYPE", "FIXED"),
                (attr "IDELAY_MODE", "PCI")
            ]);
            fuzz_multi_attr_dec!(ctx, "IDELAY2_VALUE", 8, [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2"),
                (attr "IDELAY_TYPE", "FIXED"),
                (attr "IDELAY_MODE", "PCI")
            ]);
            fuzz_multi_attr_dec!(ctx, "MCB_ADDRESS", 4, [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (global_mutex "DRPSDO", "NOPE"),
                (mode "IODRP2_MCB")
            ]);
            fuzz_one!(ctx, "ENABLE.CIN", "1", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2"),
                (pin "CIN")
            ], [
                (pin_pips "CIN")
            ]);
            fuzz_enum!(ctx, "TEST_GLITCH_FILTER", ["FALSE", "TRUE"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2")
            ]);
            fuzz_enum!(ctx, "COUNTER_WRAPAROUND", ["WRAPAROUND", "STAY_AT_LIMIT"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2")
            ]);
            fuzz_enum!(ctx, "IODELAY_CHANGE", ["CHANGE_ON_CLOCK", "CHANGE_ON_DATA"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2")
            ]);

            fuzz_enum!(ctx, "IDELAY_TYPE", ["FIXED", "DEFAULT", "VARIABLE_FROM_ZERO", "VARIABLE_FROM_HALF_MAX", "DIFF_PHASE_DETECTOR"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2"),
                (bel_unused bel_other)
            ]);
            fuzz_enum_suffix!(ctx, "IDELAY_TYPE", "DPD", ["FIXED", "DEFAULT", "VARIABLE_FROM_ZERO", "VARIABLE_FROM_HALF_MAX", "DIFF_PHASE_DETECTOR"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2"),
                (bel_mode bel_other, "IODELAY2"),
                (bel_attr bel_other, "IDELAY_TYPE", "DIFF_PHASE_DETECTOR")
            ]);

            fuzz_one!(ctx, "ENABLE.ODATAIN", "1", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2")
            ], [
                (pip (bel_pin bel_ologic, "OQ"), (pin "ODATAIN"))
            ]);

            fuzz_one!(ctx, "MUX.IOCLK", "ILOGIC_CLK", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "IODELAY"),
                (mutex "MUX.IOCLK", "ILOGIC_CLK"),
                (mode "IODELAY2"),
                (pip (bel_pin bel_ioiclk, "CLK0_ILOGIC"), (bel_pin bel_ilogic, "CLK0"))
            ], [
                (pip (bel_pin bel_ioiclk, "CLK0_ILOGIC"), (pin "IOCLK0"))
            ]);
            fuzz_one!(ctx, "MUX.IOCLK", "OLOGIC_CLK", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "IODELAY"),
                (mutex "MUX.IOCLK", "OLOGIC_CLK"),
                (mode "IODELAY2"),
                (pip (bel_pin bel_ioiclk, "CLK0_OLOGIC"), (bel_pin bel_ologic, "CLK0"))
            ], [
                (pip (bel_pin bel_ioiclk, "CLK0_OLOGIC"), (pin "IOCLK0"))
            ]);

            fuzz_enum!(ctx, "DELAY_SRC", ["IDATAIN", "ODATAIN", "IO"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODRP2"),
                (attr "IDELAY_MODE", "NORMAL")
            ]);
            fuzz_enum!(ctx, "IDELAY_MODE", ["PCI", "NORMAL"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2")
            ]);
            fuzz_enum!(ctx, "DELAYCHAIN_OSC", ["FALSE", "TRUE"], [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (mode "IODELAY2")
            ]);
        }
        for i in 0..2 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("IOICLK{i}"),
                TileBits::MainAuto,
            );
            let bel_ilogic = BelId::from_idx(i);
            let bel_ologic = BelId::from_idx(2 + i);
            let bel_ioi = BelId::from_idx(9);
            for (j, pin) in [(0, "CKINT0"), (0, "CKINT1"), (1, "CKINT0"), (1, "CKINT1")] {
                fuzz_one!(ctx, format!("MUX.CLK{j}"), pin, [
                    (related TileRelation::Delta(0, 0, node_iob), (nop)),
                    (mutex format!("MUX.CLK{j}"), pin),
                    (tile_mutex "CLK", "TEST_INTER")
                ], [
                    (pip (pin pin), (pin format!("CLK{j}INTER")))
                ]);
            }
            for (j, pin) in [
                (0, "IOCLK0"),
                (0, "IOCLK2"),
                (0, "PLLCLK0"),
                (1, "IOCLK1"),
                (1, "IOCLK3"),
                (1, "PLLCLK1"),
                (2, "PLLCLK0"),
                (2, "PLLCLK1"),
            ] {
                fuzz_one!(ctx, format!("MUX.CLK{j}"), pin, [
                    (related TileRelation::Delta(0, 0, node_iob), (nop)),
                    (mutex format!("MUX.CLK{j}"), pin),
                    (tile_mutex "CLK", "TEST_INTER")
                ], [
                    (pip (bel_pin bel_ioi, pin), (pin format!("CLK{j}INTER")))
                ]);
            }
            for j in 0..3 {
                fuzz_one!(ctx, format!("INV.CLK{j}"), "1", [
                    (related TileRelation::Delta(0, 0, node_iob), (nop)),
                    (tile_mutex "CLK", "TEST_INV"),
                    (pip (pin format!("CLK{j}INTER")), (pin "CLK0_ILOGIC")),
                    (pip (pin "CLK0_ILOGIC"), (bel_pin bel_ilogic, "CLK0")),
                    (bel_mode bel_ilogic, "ISERDES2"),
                    (bel_attr bel_ilogic, "DATA_RATE", "SDR"),
                    (bel_pin bel_ilogic, "CLK0")
                ], [
                    (bel_attr bel_ilogic, "CLK0INV", "CLK0_B")
                ]);
            }
            for j in 0..3 {
                fuzz_one!(ctx, "MUX.ICLK", format!("CLK{j}"), [
                    (related TileRelation::Delta(0, 0, node_iob), (nop)),
                    (tile_mutex "CLK", "TEST_CLK"),
                    (mutex "MUX.ICLK", format!("CLK{j}")),
                    (pip (pin "CLK0_ILOGIC"), (bel_pin bel_ilogic, "CLK0")),
                    (bel_mode bel_ilogic, "ISERDES2"),
                    (bel_attr bel_ilogic, "DATA_RATE", "SDR"),
                    (bel_pin bel_ilogic, "CLK0")
                ], [
                    (pip (pin format!("CLK{j}INTER")), (pin "CLK0_ILOGIC"))
                ]);
                fuzz_one!(ctx, "MUX.OCLK", format!("CLK{j}"), [
                    (related TileRelation::Delta(0, 0, node_iob), (nop)),
                    (tile_mutex "CLK", "TEST_CLK"),
                    (mutex "MUX.OCLK", format!("CLK{j}")),
                    (pip (pin "CLK0_OLOGIC"), (bel_pin bel_ologic, "CLK0")),
                    (bel_mode bel_ologic, "OSERDES2"),
                    (bel_attr bel_ologic, "DATA_RATE_OQ", "SDR"),
                    (bel_pin bel_ologic, "CLK0")
                ], [
                    (pip (pin format!("CLK{j}INTER")), (pin "CLK0_OLOGIC"))
                ]);
            }
            fuzz_one!(ctx, "MUX.ICLK", "DDR", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_ICLK_DDR"),
                (mutex "MUX.ICLK", "DDR"),
                (pip (pin "CLK0_ILOGIC"), (bel_pin bel_ilogic, "CLK0")),
                (bel_mode bel_ilogic, "ISERDES2"),
                (bel_attr bel_ilogic, "DATA_RATE", "DDR"),
                (bel_pin bel_ilogic, "CLK0")
            ], [
                (pip (pin "CLK0INTER"), (pin "CLK0_ILOGIC"))
            ]);
            fuzz_one!(ctx, "MUX.OCLK", "DDR", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_OCLK_DDR"),
                (mutex "MUX.OCLK", "DDR"),
                (pip (pin "CLK0_OLOGIC"), (bel_pin bel_ologic, "CLK0")),
                (bel_mode bel_ologic, "OSERDES2"),
                (bel_attr bel_ologic, "DATA_RATE_OQ", "DDR"),
                (bel_pin bel_ologic, "CLK0")
            ], [
                (pip (pin "CLK0INTER"), (pin "CLK0_OLOGIC"))
            ]);
            fuzz_one!(ctx, "MUX.ICLK", "DDR.ILOGIC", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_ICLK_DDR"),
                (mutex "MUX.ICLK", "DDR"),
                (pip (pin "CLK0_ILOGIC"), (bel_pin bel_ilogic, "CLK0")),
                (bel_mode bel_ilogic, "ILOGIC2"),
                (bel_attr bel_ilogic, "IFFTYPE", "DDR"),
                (bel_attr bel_ilogic, "DDR_ALIGNMENT", ""),
                (bel_pin bel_ilogic, "CLK0")
            ], [
                (pip (pin "CLK0INTER"), (pin "CLK0_ILOGIC"))
            ]);
            fuzz_one!(ctx, "MUX.ICLK", "DDR.ILOGIC.C0", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_ICLK_DDR_C0"),
                (mutex "MUX.ICLK", "DDR"),
                (pip (pin "CLK0_ILOGIC"), (bel_pin bel_ilogic, "CLK0")),
                (bel_mode bel_ilogic, "ILOGIC2"),
                (bel_attr bel_ilogic, "IFFTYPE", "DDR"),
                (bel_attr bel_ilogic, "DDR_ALIGNMENT", "C0"),
                (bel_pin bel_ilogic, "CLK0")
            ], [
                (pip (pin "CLK0INTER"), (pin "CLK0_ILOGIC")),
                (pip (pin "CLK1INTER"), (pin "CLK1"))
            ]);
            fuzz_one!(ctx, "MUX.ICLK", "DDR.ILOGIC.C1", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_ICLK_DDR_C1"),
                (mutex "MUX.ICLK", "DDR"),
                (pip (pin "CLK0_ILOGIC"), (bel_pin bel_ilogic, "CLK0")),
                (bel_mode bel_ilogic, "ILOGIC2"),
                (bel_attr bel_ilogic, "IFFTYPE", "DDR"),
                (bel_attr bel_ilogic, "DDR_ALIGNMENT", "C0"),
                (bel_pin bel_ilogic, "CLK0")
            ], [
                (pip (pin "CLK1INTER"), (pin "CLK0_ILOGIC")),
                (pip (pin "CLK0INTER"), (pin "CLK1"))
            ]);
            fuzz_one!(ctx, "MUX.OCLK", "DDR.OLOGIC", [
                (related TileRelation::Delta(0, 0, node_iob), (nop)),
                (tile_mutex "CLK", "TEST_OCLK_DDR"),
                (mutex "MUX.OCLK", "DDR"),
                (pip (pin "CLK0_OLOGIC"), (bel_pin bel_ologic, "CLK0")),
                (bel_mode bel_ologic, "OLOGIC2"),
                (bel_attr bel_ologic, "OUTFFTYPE", "DDR"),
                (bel_attr bel_ologic, "TFFTYPE", "DDR"),
                (bel_attr bel_ologic, "ODDR_ALIGNMENT", ""),
                (bel_attr bel_ologic, "TDDR_ALIGNMENT", ""),
                (bel_pin bel_ologic, "CLK0")
            ], [
                (pip (pin "CLK0INTER"), (pin "CLK0_OLOGIC"))
            ]);
            for j in 0..2 {
                for pin in ["IOCE0", "IOCE1", "IOCE2", "IOCE3", "PLLCE0", "PLLCE1"] {
                    fuzz_one!(ctx, ["MUX.ICE", "MUX.OCE"][j], pin, [
                        (related TileRelation::Delta(0, 0, node_iob), (nop)),
                        (tile_mutex "CLK", ["TEST_ICE", "TEST_OCE"][j]),
                        (mutex ["MUX.ICE", "MUX.OCE"][j], pin)
                    ], [
                        (pip (bel_pin bel_ioi, pin), (pin format!("IOCE{j}")))
                    ]);
                }
            }
        }
        let ctx = FuzzCtx::new(session, backend, tile, "IOI", TileBits::MainAuto);
        if tile == "IOI.BT" {
            let bel_iodelay = BelId::from_idx(4);
            fuzz_one!(ctx, "DRPSDO", "1", [
                (global_mutex "MCB", "NONE"),
                (global_mutex "DRPSDO", "TEST"),
                (global_opt "MEM_PLL_POL_SEL", "INVERTED"),
                (global_opt "MEM_PLL_DIV_EN", "DISABLED")
            ], [
                (pip (pin "MCB_DRPSDO"), (bel_pin bel_iodelay, "CE"))
            ]);
            fuzz_one!(ctx, "DRPSDO", "1.DIV_EN", [
                (global_mutex "MCB", "NONE"),
                (global_mutex "DRPSDO", "TEST"),
                (global_opt "MEM_PLL_POL_SEL", "INVERTED"),
                (global_opt "MEM_PLL_DIV_EN", "ENABLED")
            ], [
                (pip (pin "MCB_DRPSDO"), (bel_pin bel_iodelay, "CE"))
            ]);
            fuzz_one!(ctx, "DRPSDO", "1.NOTINV", [
                (global_mutex "MCB", "NONE"),
                (global_mutex "DRPSDO", "TEST"),
                (global_opt "MEM_PLL_POL_SEL", "NOTINVERTED"),
                (global_opt "MEM_PLL_DIV_EN", "DISABLED")
            ], [
                (pip (pin "MCB_DRPSDO"), (bel_pin bel_iodelay, "CE"))
            ]);
        }
    }
    if let Some(ctx) = FuzzCtx::try_new(session, backend, "MCB", "MCB", TileBits::Null) {
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::AllMcbIoi,
            "IOI.LR",
            "IOI",
            "DRPSDO",
            "1",
        )];
        fuzz_one_extras!(ctx, "DRPSDO", "1", [
            (global_mutex "MCB", "NONE"),
            (global_mutex "DRPSDO", "TEST"),
            (global_opt "MEM_PLL_POL_SEL", "INVERTED"),
            (global_opt "MEM_PLL_DIV_EN", "DISABLED")
        ], [
            (pip (pin "IOIDRPSDO"), (pin_far "IOIDRPSDO"))
        ], extras);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::AllMcbIoi,
            "IOI.LR",
            "IOI",
            "DRPSDO",
            "1.DIV_EN",
        )];
        fuzz_one_extras!(ctx, "DRPSDO", "1", [
            (global_mutex "MCB", "NONE"),
            (global_mutex "DRPSDO", "TEST"),
            (global_opt "MEM_PLL_POL_SEL", "INVERTED"),
            (global_opt "MEM_PLL_DIV_EN", "ENABLED")
        ], [
            (pip (pin "IOIDRPSDO"), (pin_far "IOIDRPSDO"))
        ], extras);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::AllMcbIoi,
            "IOI.LR",
            "IOI",
            "DRPSDO",
            "1.NOTINV",
        )];
        fuzz_one_extras!(ctx, "DRPSDO", "1", [
            (global_mutex "MCB", "NONE"),
            (global_mutex "DRPSDO", "TEST"),
            (global_opt "MEM_PLL_POL_SEL", "NOTINVERTED"),
            (global_opt "MEM_PLL_DIV_EN", "DISABLED")
        ], [
            (pip (pin "IOIDRPSDO"), (pin_far "IOIDRPSDO"))
        ], extras);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["IOI.LR", "IOI.BT"] {
        for i in 0..2 {
            let bel = &format!("ILOGIC{i}");
            ctx.state
                .get_diff(tile, bel, "MODE", "ILOGIC2")
                .assert_empty();
            // TODO: wtf is this bit really? could be MUX.IOCE...
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE.IOCE", "1");
            let diff = ctx.state.get_diff(tile, bel, "MUX.CLK", format!("ICLK{i}"));
            assert_eq!(diff.bits.len(), 1);
            let mut diff2 = Diff::default();
            for (&k, &v) in &diff.bits {
                diff2
                    .bits
                    .insert(FeatureBit::new(k.tile, k.frame, k.bit ^ 1), v);
            }
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.CLK",
                xlat_enum_ocd(
                    vec![
                        ("NONE".to_string(), Diff::default()),
                        (format!("ICLK{i}"), diff),
                        (format!("ICLK{}", i ^ 1), diff2),
                    ],
                    OcdMode::BitOrder,
                ),
            );

            ctx.collect_enum_bool(tile, bel, "BITSLIP_ENABLE", "FALSE", "TRUE");
            let item = ctx.extract_bit(tile, bel, "SRUSED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_SR_USED", item);
            let item = ctx.extract_bit(tile, bel, "REVUSED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_REV_USED", item);
            let item = ctx.extract_enum_bool(tile, bel, "SRTYPE_Q", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "IFF_SR_SYNC", item);
            ctx.state
                .get_diff(tile, bel, "SRINIT_Q", "0")
                .assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "SRINIT_Q", "1");
            let diff_init = diff.split_bits_by(|bit| matches!(bit.bit, 38 | 41));
            ctx.tiledb.insert(tile, bel, "IFF_SRVAL", xlat_bit(diff));
            ctx.tiledb
                .insert(tile, bel, "IFF_INIT", xlat_bit(diff_init));
            ctx.collect_bit(tile, bel, "IFF_CE_ENABLE", "0");
            let item = ctx.extract_enum(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
            ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);

            ctx.collect_enum(tile, bel, "MUX.SR", &["INT", "OLOGIC_SR"]);

            if i == 1 {
                ctx.collect_enum_default(tile, bel, "MUX.D", &["OTHER_IOB_I"], "IOB_I");
            }

            let mut serdes = ctx.state.get_diff(tile, bel, "MODE", "ISERDES2");
            let mut diff_ff = ctx.state.get_diff(tile, bel, "IFFTYPE", "#FF");
            let diff_latch = ctx
                .state
                .get_diff(tile, bel, "IFFTYPE", "#LATCH")
                .combine(&!&diff_ff);
            let mut diff_ddr = ctx.state.get_diff(tile, bel, "IFFTYPE", "DDR");
            ctx.tiledb
                .insert(tile, bel, "IFF_LATCH", xlat_bit(diff_latch));

            diff_ff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_CE_ENABLE"), false, true);
            diff_ff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
            diff_ff.assert_empty();
            diff_ddr.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_CE_ENABLE"), false, true);

            let mut diff_n = ctx
                .state
                .get_diff(tile, bel, "INTERFACE_TYPE", "NETWORKING");
            let mut diff_np =
                ctx.state
                    .get_diff(tile, bel, "INTERFACE_TYPE", "NETWORKING_PIPELINED");
            let mut diff_r = ctx.state.get_diff(tile, bel, "INTERFACE_TYPE", "RETIMED");
            for (attr, range) in [
                ("MUX.Q1", 46..50),
                ("MUX.Q2", 44..52),
                ("MUX.Q3", 42..54),
                ("MUX.Q4", 40..56),
            ] {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    attr,
                    xlat_enum(vec![
                        ("SHIFT_REGISTER", Diff::default()),
                        (
                            "NETWORKING",
                            diff_n.split_bits_by(|bit| range.contains(&bit.bit)),
                        ),
                        (
                            "NETWORKING_PIPELINED",
                            diff_np.split_bits_by(|bit| range.contains(&bit.bit)),
                        ),
                        (
                            "RETIMED",
                            diff_r.split_bits_by(|bit| range.contains(&bit.bit)),
                        ),
                    ]),
                );
            }
            diff_n.assert_empty();
            diff_np.assert_empty();
            diff_r.assert_empty();

            let mut diff_1 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "1");
            let mut diff_2 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "2");
            let mut diff_3 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "3");
            let mut diff_4 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "4");
            let mut diff_5 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "5");
            let mut diff_6 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "6");
            let mut diff_7 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "7");
            let mut diff_8 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "8");
            let mut diff_1_f = Diff::default();
            let mut diff_2_f = Diff::default();
            let mut diff_3_f = Diff::default();
            let mut diff_4_f = Diff::default();
            for (diff, diff_f) in [
                (&mut diff_1, &mut diff_1_f),
                (&mut diff_2, &mut diff_2_f),
                (&mut diff_3, &mut diff_3_f),
                (&mut diff_4, &mut diff_4_f),
            ] {
                diff.bits.retain(|k, v| {
                    if !*v {
                        diff_f.bits.insert(*k, *v);
                    }
                    *v
                });
            }
            diff_1_f = diff_1_f.combine(&!&diff_2_f);
            diff_2_f = diff_2_f.combine(&!&diff_3_f);
            diff_3_f = diff_3_f.combine(&!&diff_4_f);

            if i == 1 {
                serdes = serdes.combine(&diff_4_f);
                ctx.tiledb
                    .insert(tile, bel, "CASCADE_ENABLE", xlat_bit(!diff_4_f));
            } else {
                diff_4_f.assert_empty();
            }

            serdes = serdes
                .combine(&diff_1_f)
                .combine(&diff_2_f)
                .combine(&diff_3_f);
            diff_ddr = diff_ddr.combine(&diff_1_f);
            ctx.tiledb
                .insert(tile, bel, "ROW2_CLK_ENABLE", xlat_bit(!diff_1_f));
            ctx.tiledb
                .insert(tile, bel, "ROW3_CLK_ENABLE", xlat_bit(!diff_2_f));
            ctx.tiledb
                .insert(tile, bel, "ROW4_CLK_ENABLE", xlat_bit(!diff_3_f));

            let (serdes, mut diff_ddr, diff_row1) = Diff::split(serdes, diff_ddr);
            ctx.tiledb
                .insert(tile, bel, "ROW1_CLK_ENABLE", xlat_bit(diff_row1));

            serdes.assert_empty();

            let diff_1_a = diff_1.split_bits_by(|bit| bit.frame == 27);
            let diff_2_a = diff_2.split_bits_by(|bit| bit.frame == 27);
            let diff_3_a = diff_3.split_bits_by(|bit| bit.frame == 27);
            let diff_4_a = diff_4.split_bits_by(|bit| bit.frame == 27);
            let diff_5_a = diff_5.split_bits_by(|bit| bit.frame == 27);
            let diff_6_a = diff_6.split_bits_by(|bit| bit.frame == 27);
            let diff_7_a = diff_7.split_bits_by(|bit| bit.frame == 27);
            let diff_8_a = diff_8.split_bits_by(|bit| bit.frame == 27);

            assert_eq!(diff_1, diff_2);
            if i == 0 {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "DATA_WIDTH_RELOAD",
                    xlat_enum(vec![
                        ("8", diff_8_a),
                        ("7", diff_7_a),
                        ("6", diff_6_a),
                        ("5", diff_5_a),
                        ("4", diff_4_a),
                        ("3", diff_3_a),
                        ("2", diff_2_a),
                        ("1", diff_1_a),
                    ]),
                );
                let (diff_5, diff_6, diff_casc) = Diff::split(diff_5, diff_6);
                let diff_7 = diff_7.combine(&!&diff_casc);
                let diff_8 = diff_8.combine(&!&diff_casc);
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "DATA_WIDTH_START",
                    xlat_enum(vec![
                        ("2", diff_2),
                        ("3", diff_3),
                        ("4", diff_4),
                        ("5", diff_5),
                        ("6", diff_6),
                        ("7", diff_7),
                        ("8", diff_8),
                    ]),
                );
                ctx.tiledb
                    .insert(tile, bel, "CASCADE_ENABLE", xlat_bit(diff_casc));
                diff_ddr.apply_enum_diff(ctx.tiledb.item(tile, bel, "DATA_WIDTH_RELOAD"), "2", "8");
            } else {
                assert_eq!(diff_3_a, diff_5_a);
                assert_eq!(diff_3_a, diff_6_a);
                assert_eq!(diff_3_a, diff_7_a);
                assert_eq!(diff_3_a, diff_8_a);
                diff_5.assert_empty();
                diff_6.assert_empty();
                diff_7.assert_empty();
                diff_8.assert_empty();
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "DATA_WIDTH_RELOAD",
                    xlat_enum(vec![
                        ("4", diff_4_a),
                        ("3", diff_3_a),
                        ("2", diff_2_a),
                        ("1", diff_1_a),
                    ]),
                );
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "DATA_WIDTH_START",
                    xlat_enum(vec![("2", diff_2), ("3", diff_3), ("4", diff_4)]),
                );
                diff_ddr.apply_enum_diff(ctx.tiledb.item(tile, bel, "DATA_WIDTH_RELOAD"), "2", "4");
            }
            diff_ddr.apply_enum_diff(ctx.tiledb.item(tile, bel, "DATA_WIDTH_START"), "3", "2");

            ctx.tiledb.insert(tile, bel, "DDR", xlat_bit(diff_ddr));
        }
        for i in 0..2 {
            let bel = &format!("OLOGIC{i}");
            ctx.state
                .get_diff(tile, bel, "MODE", "OLOGIC2")
                .assert_empty();
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE.IOCE", "1");
            let diff = ctx.state.get_diff(tile, bel, "MUX.CLK", format!("OCLK{i}"));
            assert_eq!(diff.bits.len(), 1);
            let mut diff2 = Diff::default();
            for (&k, &v) in &diff.bits {
                diff2
                    .bits
                    .insert(FeatureBit::new(k.tile, k.frame, k.bit ^ 1), v);
            }
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.CLK",
                xlat_enum_ocd(
                    vec![
                        ("NONE".to_string(), Diff::default()),
                        (format!("OCLK{i}"), diff),
                        (format!("OCLK{}", i ^ 1), diff2),
                    ],
                    OcdMode::BitOrder,
                ),
            );

            for (attr, sattr) in [
                ("OFF_SR_ENABLE", "OSRUSED"),
                ("TFF_SR_ENABLE", "TSRUSED"),
                ("OFF_REV_ENABLE", "OREVUSED"),
                ("TFF_REV_ENABLE", "TREVUSED"),
            ] {
                let item = ctx.extract_bit(tile, bel, sattr, "0");
                ctx.tiledb.insert(tile, bel, attr, item);
            }
            for attr in ["MUX.REV", "MUX.SR"] {
                ctx.collect_enum_default(tile, bel, attr, &["INT"], "GND");
            }
            let item = ctx.extract_enum_bool(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "OFF_SR_SYNC", item);
            let item = ctx.extract_enum_bool(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "TFF_SR_SYNC", item);

            ctx.collect_bitvec(tile, bel, "TRAIN_PATTERN", "");
            ctx.collect_enum_default(tile, bel, "MUX.TRAIN", &["INT", "MCB"], "GND");
            let item = ctx.extract_bit(tile, bel, "MISRATTRBOX", "MISR_ENABLE_DATA");
            ctx.tiledb.insert(tile, bel, "MISR_ENABLE_DATA", item);
            let item = ctx.extract_bit(tile, bel, "MODE", "OLOGIC2.MISR_RESET");
            ctx.tiledb.insert(tile, bel, "MISR_RESET", item);
            for val in ["CLK0", "CLK1"] {
                let item = ctx.extract_bit(tile, bel, "MISR_ENABLE_CLK", val);
                ctx.tiledb.insert(tile, bel, "MISR_ENABLE_CLK", item);
            }
            for val in ["1", "2", "3", "4"] {
                ctx.state
                    .get_diff(tile, bel, "DATA_WIDTH", val)
                    .assert_empty();
            }
            for val in ["5", "6", "7", "8"] {
                let item = ctx.extract_bit(tile, bel, "DATA_WIDTH", val);
                ctx.tiledb.insert(tile, bel, "CASCADE_ENABLE", item);
            }
            if i == 0 {
                ctx.state
                    .get_diff(tile, bel, "OUTPUT_MODE", "SINGLE_ENDED")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, bel, "OUTPUT_MODE", "DIFFERENTIAL")
                    .assert_empty();
            } else {
                ctx.collect_enum(tile, bel, "OUTPUT_MODE", &["SINGLE_ENDED", "DIFFERENTIAL"]);
            }

            let mut serdes = ctx.state.get_diff(tile, bel, "MODE", "OSERDES2");
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);

            ctx.state
                .get_diff(tile, bel, "SRINIT_OQ", "0")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "SRINIT_TQ", "0")
                .assert_empty();
            let diff = ctx.state.get_diff(tile, bel, "SRINIT_TQ", "1");
            let (mut serdes, diff_init, diff_srval) = Diff::split(serdes, diff);
            ctx.tiledb
                .insert(tile, bel, "TFF_INIT", xlat_bit(diff_init));
            ctx.tiledb
                .insert(tile, bel, "TFF_SRVAL", xlat_bit(diff_srval));
            let mut diff = ctx.state.get_diff(tile, bel, "SRINIT_OQ", "1");
            let diff_srval = diff.split_bits_by(|bit| matches!(bit.bit, 8 | 24));
            ctx.tiledb.insert(tile, bel, "OFF_INIT", xlat_bit(diff));
            ctx.tiledb
                .insert(tile, bel, "OFF_SRVAL", xlat_bit(diff_srval));

            let mut diff = ctx.state.get_diff(tile, bel, "MUX.D", "MCB");
            let diff_t = diff.split_bits_by(|bit| matches!(bit.bit, 2 | 28));
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.T",
                xlat_enum(vec![("INT", Diff::default()), ("MCB", diff_t)]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.D",
                xlat_enum(vec![("INT", Diff::default()), ("MCB", diff)]),
            );

            ctx.collect_enum(tile, bel, "OMUX", &["D1", "OUTFF"]);
            let diff = ctx.state.get_diff(tile, bel, "OT1USED", "0");
            ctx.tiledb.insert(
                tile,
                bel,
                "TMUX",
                xlat_enum(vec![("TFF", Diff::default()), ("T1", diff)]),
            );

            let item = ctx.extract_bit(tile, bel, "DDR_ALIGNMENT", "NONE");
            ctx.tiledb.insert(tile, bel, "DDR_OPPOSITE_EDGE", item);
            let item = ctx.extract_bit(tile, bel, "TDDR_ALIGNMENT", "NONE");
            ctx.tiledb.insert(tile, bel, "DDR_OPPOSITE_EDGE", item);

            let item = ctx.extract_bit(tile, bel, "DDR_ALIGNMENT", "C0");
            ctx.tiledb.insert(tile, bel, "OFF_RANK2_CLK_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "TDDR_ALIGNMENT", "C0");
            ctx.tiledb.insert(tile, bel, "TFF_RANK2_CLK_ENABLE", item);

            let mut diff = ctx.state.get_diff(tile, bel, "BYPASS_GCLK_FF", "FALSE");
            let diff_t = diff.split_bits_by(|bit| matches!(bit.bit, 6 | 22));
            ctx.tiledb
                .insert(tile, bel, "OFF_RANK1_CLK_ENABLE", xlat_bit(diff));
            ctx.tiledb
                .insert(tile, bel, "TFF_RANK1_CLK_ENABLE", xlat_bit(diff_t));

            let diff_bypass = ctx.state.get_diff(tile, bel, "BYPASS_GCLK_FF", "TRUE");
            let diff_olatch = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#LATCH");
            let diff_off = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#FF");
            let diff_oddr = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "DDR");
            let diff_tlatch = ctx.state.get_diff(tile, bel, "TFFTYPE", "#LATCH");
            let diff_tff = ctx.state.get_diff(tile, bel, "TFFTYPE", "#FF");
            let diff_tddr = ctx.state.get_diff(tile, bel, "TFFTYPE", "DDR");
            let diff_oce = ctx.state.get_diff(tile, bel, "MUX.OCE", "INT");
            let diff_oce_pci = ctx.state.get_diff(tile, bel, "MUX.OCE", "PCI_CE");
            let diff_tce = ctx.state.get_diff(tile, bel, "MUX.TCE", "INT");

            let diff_oce_pci = diff_oce_pci.combine(&!&diff_oce);
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.OCE",
                xlat_enum(vec![("INT", Diff::default()), ("PCI_CE", diff_oce_pci)]),
            );

            let diff_tlatch = diff_tlatch.combine(&!&diff_tff);
            let diff_olatch = diff_olatch.combine(&!&diff_tlatch).combine(&!&diff_off);
            ctx.tiledb
                .insert(tile, bel, "TFF_LATCH", xlat_bit(diff_tlatch));
            ctx.tiledb
                .insert(tile, bel, "OFF_LATCH", xlat_bit(diff_olatch));

            let (diff_tff, diff_obypass, diff_tbypass) = Diff::split(diff_tff, diff_bypass);
            let diff_tddr = diff_tddr.combine(&!&diff_tbypass);
            let diff_off = diff_off.combine(&!&diff_obypass);
            let diff_oddr = diff_oddr.combine(&!&diff_obypass);
            ctx.tiledb
                .insert(tile, bel, "OFF_RANK1_BYPASS", xlat_bit(diff_obypass));
            ctx.tiledb
                .insert(tile, bel, "TFF_RANK1_BYPASS", xlat_bit(diff_tbypass));

            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff_off));
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff_tff));

            let diff_oce = diff_oce.combine(&!&diff_oddr);
            let diff_tce = diff_tce.combine(&!&diff_tddr);
            ctx.tiledb
                .insert(tile, bel, "OFF_CE_OR_DDR", xlat_bit(diff_oddr));
            ctx.tiledb
                .insert(tile, bel, "TFF_CE_OR_DDR", xlat_bit(diff_tddr));

            ctx.tiledb
                .insert(tile, bel, "OFF_CE_ENABLE", xlat_bit(diff_oce));
            ctx.tiledb
                .insert(tile, bel, "TFF_CE_ENABLE", xlat_bit(diff_tce));

            serdes.apply_bit_diff(
                ctx.tiledb.item(tile, bel, "OFF_RANK2_CLK_ENABLE"),
                true,
                false,
            );
            serdes.apply_bit_diff(
                ctx.tiledb.item(tile, bel, "TFF_RANK2_CLK_ENABLE"),
                true,
                false,
            );
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_CE_ENABLE"), true, false);
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_CE_ENABLE"), true, false);
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_CE_OR_DDR"), true, false);
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_CE_OR_DDR"), true, false);

            serdes.assert_empty();
        }
        let (_, _, diff) = Diff::split(
            ctx.state
                .peek_diff(tile, "IODELAY0", "MODE", "IODRP2")
                .clone(),
            ctx.state
                .peek_diff(tile, "IODELAY1", "MODE", "IODRP2")
                .clone(),
        );
        let (_, _, diff_mcb) = Diff::split(
            ctx.state
                .peek_diff(tile, "IODELAY0", "MODE", "IODRP2_MCB")
                .clone(),
            ctx.state
                .peek_diff(tile, "IODELAY1", "MODE", "IODRP2_MCB")
                .clone(),
        );
        let diff_mcb = diff_mcb.combine(&!&diff);
        ctx.tiledb
            .insert(tile, "IODELAY_COMMON", "DRP_ENABLE", xlat_bit(diff));
        ctx.tiledb
            .insert(tile, "IODELAY_COMMON", "DRP_FROM_MCB", xlat_bit(diff_mcb));

        for i in 0..2 {
            let bel = &format!("IODELAY{i}");
            let diffs = ctx.state.get_diffs(tile, bel, "ODELAY_VALUE", "");
            let mut diffs_p = vec![];
            let mut diffs_n = vec![];
            for mut diff in diffs {
                let diff_p = diff.split_bits_by(|bit| (16..48).contains(&bit.bit));
                diffs_p.push(diff_p);
                diffs_n.push(diff);
            }
            ctx.tiledb
                .insert(tile, bel, "ODELAY_VALUE_P", xlat_bitvec(diffs_p));
            ctx.tiledb
                .insert(tile, bel, "ODELAY_VALUE_N", xlat_bitvec(diffs_n));
            let item = ctx.extract_bitvec(tile, bel, "IDELAY_VALUE", "");
            ctx.tiledb.insert(tile, bel, "IDELAY_VALUE_P", item);
            let item = ctx.extract_bitvec(tile, bel, "IDELAY2_VALUE", "");
            ctx.tiledb.insert(tile, bel, "IDELAY_VALUE_N", item);
            if i == 0 {
                let item = ctx.extract_bitvec(tile, bel, "MCB_ADDRESS", "");
                ctx.tiledb
                    .insert(tile, "IODELAY_COMMON", "MCB_ADDRESS", item);
            } else {
                let diffs = ctx.state.get_diffs(tile, bel, "MCB_ADDRESS", "");
                for diff in diffs {
                    diff.assert_empty();
                }
            }
            ctx.collect_bit_wide(tile, bel, "ENABLE.CIN", "1");
            ctx.collect_enum_bool(tile, bel, "TEST_GLITCH_FILTER", "FALSE", "TRUE");
            ctx.collect_enum(
                tile,
                bel,
                "COUNTER_WRAPAROUND",
                &["WRAPAROUND", "STAY_AT_LIMIT"],
            );
            ctx.collect_enum(
                tile,
                bel,
                "IODELAY_CHANGE",
                &["CHANGE_ON_CLOCK", "CHANGE_ON_DATA"],
            );
            let diff = ctx
                .state
                .get_diff(tile, bel, "MODE", "IODELAY2.TEST_NCOUNTER")
                .combine(&!ctx.state.peek_diff(tile, bel, "MODE", "IODELAY2"));
            ctx.tiledb
                .insert(tile, bel, "TEST_NCOUNTER", xlat_bit(diff));
            let diff = ctx
                .state
                .get_diff(tile, bel, "MODE", "IODELAY2.TEST_PCOUNTER")
                .combine(&!ctx.state.peek_diff(tile, bel, "MODE", "IODELAY2"));
            ctx.tiledb
                .insert(tile, bel, "TEST_PCOUNTER", xlat_bit(diff));
            let diff = ctx
                .state
                .get_diff(tile, bel, "MODE", "IODRP2.IOIENFFSCAN_DRP")
                .combine(&!ctx.state.peek_diff(tile, bel, "MODE", "IODRP2"));
            ctx.tiledb
                .insert(tile, "IODELAY_COMMON", "ENFFSCAN_DRP", xlat_bit_wide(diff));

            ctx.collect_bit(tile, bel, "ENABLE.ODATAIN", "1");
            ctx.collect_enum(tile, bel, "MUX.IOCLK", &["ILOGIC_CLK", "OLOGIC_CLK"]);

            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE", "DEFAULT");
            ctx.tiledb.insert(tile, bel, "IDELAY_FIXED", item);
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE", "FIXED");
            ctx.tiledb.insert(tile, bel, "IDELAY_FIXED", item);
            ctx.state
                .get_diff(tile, bel, "IDELAY_TYPE", "VARIABLE_FROM_ZERO")
                .assert_empty();
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE", "VARIABLE_FROM_HALF_MAX");
            ctx.tiledb.insert(tile, bel, "IDELAY_FROM_HALF_MAX", item);
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE.DPD", "DEFAULT");
            ctx.tiledb.insert(tile, bel, "IDELAY_FIXED", item);
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE.DPD", "FIXED");
            ctx.tiledb.insert(tile, bel, "IDELAY_FIXED", item);
            ctx.state
                .get_diff(tile, bel, "IDELAY_TYPE.DPD", "VARIABLE_FROM_ZERO")
                .assert_empty();
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE.DPD", "VARIABLE_FROM_HALF_MAX");
            ctx.tiledb.insert(tile, bel, "IDELAY_FROM_HALF_MAX", item);
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE", "DIFF_PHASE_DETECTOR");
            ctx.tiledb.insert(tile, bel, "DIFF_PHASE_DETECTOR", item);

            ctx.tiledb.insert(
                tile,
                bel,
                "CAL_DELAY_MAX",
                TileItem::from_bitvec(
                    vec![
                        FeatureBit::new(0, 28, [0, 63][i]),
                        FeatureBit::new(0, 28, [1, 62][i]),
                        FeatureBit::new(0, 28, [2, 61][i]),
                        FeatureBit::new(0, 28, [3, 60][i]),
                        FeatureBit::new(0, 28, [4, 59][i]),
                        FeatureBit::new(0, 28, [5, 58][i]),
                        FeatureBit::new(0, 28, [6, 57][i]),
                        FeatureBit::new(0, 28, [7, 56][i]),
                    ],
                    false,
                ),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "DRP_ADDR",
                TileItem::from_bitvec(
                    vec![
                        FeatureBit::new(0, 28, [24, 39][i]),
                        FeatureBit::new(0, 28, [25, 38][i]),
                        FeatureBit::new(0, 28, [26, 37][i]),
                        FeatureBit::new(0, 28, [27, 36][i]),
                        FeatureBit::new(0, 28, [31, 32][i]),
                    ],
                    false,
                ),
            );
            let drp06 = TileItem::from_bitvec(
                vec![
                    FeatureBit::new(0, 28, [18, 45][i]),
                    FeatureBit::new(0, 28, [16, 47][i]),
                    FeatureBit::new(0, 28, [13, 50][i]),
                    FeatureBit::new(0, 28, [10, 53][i]),
                    FeatureBit::new(0, 28, [8, 55][i]),
                    FeatureBit::new(0, 28, [14, 49][i]),
                    FeatureBit::new(0, 28, [22, 41][i]),
                    FeatureBit::new(0, 28, [20, 43][i]),
                ],
                false,
            );
            let drp07 = TileItem::from_bitvec(
                vec![
                    FeatureBit::new(0, 28, [19, 44][i]),
                    FeatureBit::new(0, 28, [17, 46][i]),
                    FeatureBit::new(0, 28, [12, 51][i]),
                    FeatureBit::new(0, 28, [11, 52][i]),
                    FeatureBit::new(0, 28, [9, 54][i]),
                    FeatureBit::new(0, 28, [15, 48][i]),
                    FeatureBit::new(0, 28, [23, 40][i]),
                    FeatureBit::new(0, 28, [21, 42][i]),
                ],
                false,
            );
            if i == 0 {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "EVENT_SEL",
                    TileItem::from_bitvec(drp06.bits[0..2].to_vec(), false),
                );
            } else {
                ctx.tiledb
                    .insert(tile, bel, "PLUS1", TileItem::from_bit(drp06.bits[0], false));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                "LUMPED_DELAY",
                TileItem::from_bit(drp07.bits[3], false),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "LUMPED_DELAY_SELECT",
                TileItem::from_bit(drp07.bits[4], false),
            );
            ctx.tiledb.insert(tile, bel, "DRP06", drp06);
            ctx.tiledb.insert(tile, bel, "DRP07", drp07);

            ctx.collect_enum(tile, bel, "DELAY_SRC", &["IDATAIN", "ODATAIN", "IO"]);
            ctx.state
                .get_diff(tile, bel, "IDELAY_MODE", "NORMAL")
                .assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "IDELAY_MODE", "PCI");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "ODATAIN", "IO");
            ctx.tiledb.insert(
                tile,
                bel,
                "IDELAY_MODE",
                xlat_enum(vec![("NORMAL", Diff::default()), ("PCI", diff)]),
            );

            ctx.state
                .get_diff(tile, bel, "DELAYCHAIN_OSC", "FALSE")
                .assert_empty();
            let mut diff_iodelay2 = ctx.state.get_diff(tile, bel, "MODE", "IODELAY2");
            let mut diff_iodrp2 = ctx.state.get_diff(tile, bel, "MODE", "IODRP2");
            let mut diff_iodrp2_mcb = ctx.state.get_diff(tile, bel, "MODE", "IODRP2_MCB");
            let diff_delaychain_osc = ctx.state.get_diff(tile, bel, "DELAYCHAIN_OSC", "TRUE");
            diff_iodrp2.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY_COMMON", "DRP_ENABLE"),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY_COMMON", "DRP_ENABLE"),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY_COMMON", "DRP_FROM_MCB"),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "MUX.IOCLK"),
                "OLOGIC_CLK",
                "ILOGIC_CLK",
            );
            if i == 0 {
                diff_iodelay2.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "EVENT_SEL"), 3, 0);
                diff_iodrp2.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "EVENT_SEL"), 3, 0);
                diff_iodrp2_mcb.apply_bitvec_diff_int(
                    ctx.tiledb.item(tile, bel, "EVENT_SEL"),
                    3,
                    0,
                );
            }
            let (diff_iodrp2_mcb, diff_delaychain_osc, diff_common) =
                Diff::split(diff_iodrp2_mcb, diff_delaychain_osc);
            ctx.tiledb.insert(
                tile,
                bel,
                "DELAYCHAIN_OSC_OR_ODATAIN_LP_OR_IDRP2_MCB",
                xlat_bit_wide(diff_common),
            );
            ctx.tiledb
                .insert(tile, bel, "DELAYCHAIN_OSC", xlat_bit(diff_delaychain_osc));
            ctx.tiledb.insert(
                tile,
                bel,
                "MODE",
                xlat_enum(vec![
                    ("IODELAY2", diff_iodelay2),
                    ("IODRP2", diff_iodrp2),
                    ("IODRP2_MCB", diff_iodrp2_mcb),
                ]),
            );
        }
        {
            let mut diff0 =
                ctx.state
                    .get_diff(tile, "IODELAY0", "IDELAY_TYPE.DPD", "DIFF_PHASE_DETECTOR");
            let mut diff1 =
                ctx.state
                    .get_diff(tile, "IODELAY1", "IDELAY_TYPE.DPD", "DIFF_PHASE_DETECTOR");
            diff0.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY0", "DIFF_PHASE_DETECTOR"),
                true,
                false,
            );
            diff1.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY1", "DIFF_PHASE_DETECTOR"),
                true,
                false,
            );
            diff0.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY0", "IDELAY_FROM_HALF_MAX"),
                true,
                false,
            );
            diff1.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY0", "IDELAY_FROM_HALF_MAX"),
                true,
                false,
            );
            ctx.tiledb.insert(
                tile,
                "IODELAY_COMMON",
                "DIFF_PHASE_DETECTOR",
                xlat_bit(diff0),
            );
            ctx.tiledb.insert(
                tile,
                "IODELAY_COMMON",
                "DIFF_PHASE_DETECTOR",
                xlat_bit(diff1),
            );
        }
        for i in 0..2 {
            let bel = &format!("IOICLK{i}");
            ctx.collect_bit(tile, bel, "INV.CLK0", "1");
            ctx.collect_bit(tile, bel, "INV.CLK1", "1");
            ctx.collect_bit(tile, bel, "INV.CLK2", "1");
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.CLK0",
                &["IOCLK0", "IOCLK2", "PLLCLK0", "CKINT0", "CKINT1"],
                "NONE",
            );
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.CLK1",
                &["IOCLK1", "IOCLK3", "PLLCLK1", "CKINT0", "CKINT1"],
                "NONE",
            );
            ctx.collect_enum_default(tile, bel, "MUX.CLK2", &["PLLCLK0", "PLLCLK1"], "NONE");

            let diff_iddr = ctx.state.get_diff(tile, bel, "MUX.ICLK", "DDR");
            let diff_iddr_ce = ctx.state.get_diff(tile, bel, "MUX.ICLK", "DDR.ILOGIC");
            let diff_iddr_ce_c0 = ctx.state.get_diff(tile, bel, "MUX.ICLK", "DDR.ILOGIC.C0");
            let diff_iddr_ce_c1 = ctx.state.get_diff(tile, bel, "MUX.ICLK", "DDR.ILOGIC.C1");
            let diff_oddr = ctx.state.get_diff(tile, bel, "MUX.OCLK", "DDR");
            let diff_oddr_ce = ctx.state.get_diff(tile, bel, "MUX.OCLK", "DDR.OLOGIC");
            let diff_c0 = diff_iddr_ce_c0.combine(&!&diff_iddr_ce);
            let diff_c1 = diff_iddr_ce_c1.combine(&!&diff_iddr_ce);
            let diff_iddr_ce = diff_iddr_ce.combine(&!&diff_iddr);
            let diff_oddr_ce = diff_oddr_ce.combine(&!&diff_oddr);
            let (diff_iddr, diff_oddr, diff_ddr) = Diff::split(diff_iddr, diff_oddr);
            ctx.tiledb.insert(
                tile,
                bel,
                "DDR_ALIGNMENT",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("CLK0", diff_c0),
                    ("CLK1", diff_c1),
                ]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.ICLK",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("CLK0", ctx.state.get_diff(tile, bel, "MUX.ICLK", "CLK0")),
                    ("CLK1", ctx.state.get_diff(tile, bel, "MUX.ICLK", "CLK1")),
                    ("CLK2", ctx.state.get_diff(tile, bel, "MUX.ICLK", "CLK2")),
                    ("DDR", diff_iddr),
                ]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.OCLK",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("CLK0", ctx.state.get_diff(tile, bel, "MUX.OCLK", "CLK0")),
                    ("CLK1", ctx.state.get_diff(tile, bel, "MUX.OCLK", "CLK1")),
                    ("CLK2", ctx.state.get_diff(tile, bel, "MUX.OCLK", "CLK2")),
                    ("DDR", diff_oddr),
                ]),
            );
            ctx.tiledb
                .insert(tile, bel, "DDR_ENABLE", xlat_bit_wide(diff_ddr));

            let diff_ice_ioce0 = ctx.state.get_diff(tile, bel, "MUX.ICE", "IOCE0");
            let diff_ice_ioce1 = ctx.state.get_diff(tile, bel, "MUX.ICE", "IOCE1");
            let diff_ice_ioce2 = ctx.state.get_diff(tile, bel, "MUX.ICE", "IOCE2");
            let diff_ice_ioce3 = ctx.state.get_diff(tile, bel, "MUX.ICE", "IOCE3");
            let diff_ice_pllce0 = ctx.state.get_diff(tile, bel, "MUX.ICE", "PLLCE0");
            let diff_ice_pllce1 = ctx.state.get_diff(tile, bel, "MUX.ICE", "PLLCE1");
            let diff_oce_ioce0 = ctx.state.get_diff(tile, bel, "MUX.OCE", "IOCE0");
            let diff_oce_ioce1 = ctx.state.get_diff(tile, bel, "MUX.OCE", "IOCE1");
            let diff_oce_ioce2 = ctx.state.get_diff(tile, bel, "MUX.OCE", "IOCE2");
            let diff_oce_ioce3 = ctx.state.get_diff(tile, bel, "MUX.OCE", "IOCE3");
            let diff_oce_pllce0 = ctx.state.get_diff(tile, bel, "MUX.OCE", "PLLCE0");
            let diff_oce_pllce1 = ctx.state.get_diff(tile, bel, "MUX.OCE", "PLLCE1");
            let (diff_ice_ioce0, diff_oce_ioce0, diff_ioce0) =
                Diff::split(diff_ice_ioce0, diff_oce_ioce0);
            let (diff_ice_ioce1, diff_oce_ioce1, diff_ioce1) =
                Diff::split(diff_ice_ioce1, diff_oce_ioce1);
            let (diff_ice_ioce2, diff_oce_ioce2, diff_ioce2) =
                Diff::split(diff_ice_ioce2, diff_oce_ioce2);
            let (diff_ice_ioce3, diff_oce_ioce3, diff_ioce3) =
                Diff::split(diff_ice_ioce3, diff_oce_ioce3);
            let (diff_ice_pllce0, diff_oce_pllce0, diff_pllce0) =
                Diff::split(diff_ice_pllce0, diff_oce_pllce0);
            let (diff_ice_pllce1, diff_oce_pllce1, diff_pllce1) =
                Diff::split(diff_ice_pllce1, diff_oce_pllce1);
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.ICE",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("CE0", diff_ice_ioce0),
                    ("CE0", diff_ice_ioce2),
                    ("CE0", diff_ice_pllce0),
                    ("CE1", diff_ice_ioce1),
                    ("CE1", diff_ice_ioce3),
                    ("CE1", diff_ice_pllce1),
                    ("DDR", diff_iddr_ce),
                ]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.OCE",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("CE0", diff_oce_ioce0),
                    ("CE0", diff_oce_ioce2),
                    ("CE0", diff_oce_pllce0),
                    ("CE1", diff_oce_ioce1),
                    ("CE1", diff_oce_ioce3),
                    ("CE1", diff_oce_pllce1),
                    ("DDR", diff_oddr_ce),
                ]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.CE0",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("IOCE0", diff_ioce0),
                    ("IOCE2", diff_ioce2),
                    ("PLLCE0", diff_pllce0),
                ]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.CE1",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("IOCE1", diff_ioce1),
                    ("IOCE3", diff_ioce3),
                    ("PLLCE1", diff_pllce1),
                ]),
            );
        }
        let bel = "IOI";
        if tile == "IOI.BT" || ctx.has_tile("MCB") {
            let mut diff = ctx.state.get_diff(tile, bel, "DRPSDO", "1");
            let diff_de = ctx
                .state
                .get_diff(tile, bel, "DRPSDO", "1.DIV_EN")
                .combine(&!&diff);
            let diff_ni = ctx
                .state
                .get_diff(tile, bel, "DRPSDO", "1.NOTINV")
                .combine(&!&diff);
            ctx.tiledb
                .insert(tile, bel, "MEM_PLL_DIV_EN", xlat_bit(diff_de));
            ctx.tiledb.insert(
                tile,
                bel,
                "MEM_PLL_POL_SEL",
                xlat_enum(vec![
                    ("INVERTED", Diff::default()),
                    ("NOTINVERTED", diff_ni),
                ]),
            );
            diff.apply_bitvec_diff_int(
                ctx.tiledb.item(tile, "IODELAY_COMMON", "MCB_ADDRESS"),
                0xa,
                0,
            );
            diff.assert_empty();
        }
    }
}
