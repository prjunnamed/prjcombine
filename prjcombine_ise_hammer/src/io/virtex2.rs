use bitvec::vec::BitVec;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::TileItemKind;
use prjcombine_virtex2::grid::GridKind;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{xlat_bitvec, xlat_enum, xlat_enum_default, CollectorCtx, Diff},
    fgen::{TileBits, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };
    let intdb = backend.egrid.db;

    // IOI
    for (node_kind, name, node) in &intdb.nodes {
        if !name.starts_with("IOI") {
            continue;
        }
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        for (bel_id, bel_name, bel_data) in &node.bels {
            if !bel_name.starts_with("IOI") {
                continue;
            }
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Main(1),
                tile_name: name,
                bel: bel_id,
                bel_name,
            };
            let mode = if edev.grid.kind.is_spartan3ea() {
                "IBUF"
            } else {
                "IOB"
            };

            // clock & SR invs
            fuzz_enum!(ctx, "OTCLK1INV", ["OTCLK1", "OTCLK1_B"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (pin "OTCLK1")
            ]);
            fuzz_enum!(ctx, "OTCLK2INV", ["OTCLK2", "OTCLK2_B"], [
                (mode mode),
                (attr "OFF2", "#FF"),
                (pin "OTCLK2")
            ]);
            fuzz_enum!(ctx, "ICLK1INV", ["ICLK1", "ICLK1_B"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (pin "ICLK1")
            ]);
            fuzz_enum!(ctx, "ICLK2INV", ["ICLK2", "ICLK2_B"], [
                (mode mode),
                (attr "IFF2", "#FF"),
                (pin "ICLK2")
            ]);
            fuzz_enum!(ctx, "SRINV", ["SR", "SR_B"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OSR_USED", "0"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "REVINV", ["REV", "REV_B"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OREV_USED", "0"),
                (pin "REV")
            ]);
            // SR & rev enables
            fuzz_enum!(ctx, "ISR_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "OFF1", "#FF"),
                (attr "OSR_USED", "0"),
                (attr "SRINV", "SR_B"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "OSR_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "OFF1", "#FF"),
                (attr "ISR_USED", "0"),
                (attr "SRINV", "SR_B"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "TSR_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "TFF1", "#FF"),
                (attr "ISR_USED", "0"),
                (attr "SRINV", "SR_B"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "IREV_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "OFF1", "#FF"),
                (attr "OREV_USED", "0"),
                (attr "REVINV", "REV_B"),
                (pin "REV")
            ]);
            fuzz_enum!(ctx, "OREV_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "OFF1", "#FF"),
                (attr "IREV_USED", "0"),
                (attr "REVINV", "REV_B"),
                (pin "REV")
            ]);
            fuzz_enum!(ctx, "TREV_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "TFF1", "#FF"),
                (attr "IREV_USED", "0"),
                (attr "REVINV", "REV_B"),
                (pin "REV")
            ]);

            // CE
            fuzz_enum!(ctx, "ICEINV", ["ICE", "ICE_B"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (pin "ICE")
            ]);
            fuzz_enum!(ctx, "TCEINV", ["TCE", "TCE_B"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (pin "TCE")
            ]);
            if edev.grid.kind.is_spartan3ea() {
                fuzz_enum!(ctx, "OCEINV", ["OCE", "OCE_B"], [
                    (mode mode),
                    (attr "OFF1", "#FF"),
                    (attr "PCICE_MUX", "OCE"),
                    (pin "OCE")
                ]);
                fuzz_enum!(ctx, "PCICE_MUX", ["OCE", "PCICE"], [
                    (mode mode),
                    (attr "OFF1", "#FF"),
                    (attr "OCEINV", "#OFF"),
                    (pin "OCE"),
                    (pin "PCI_CE")
                ]);
            } else {
                fuzz_enum!(ctx, "OCEINV", ["OCE", "OCE_B"], [
                    (mode mode),
                    (attr "OFF1", "#FF"),
                    (pin "OCE")
                ]);
            }
            // Output path
            if edev.grid.kind.is_spartan3ea() {
                fuzz_enum!(ctx, "O1INV", ["O1", "O1_B"], [
                    (mode mode),
                    (attr "O1_DDRMUX", "1"),
                    (attr "OFF1", "#FF"),
                    (attr "OMUX", "OFF1"),
                    (pin "O1")
                ]);
                fuzz_enum!(ctx, "O2INV", ["O2", "O2_B"], [
                    (mode mode),
                    (attr "O2_DDRMUX", "1"),
                    (attr "OFF2", "#FF"),
                    (attr "OMUX", "OFF2"),
                    (pin "O2")
                ]);
            } else {
                fuzz_enum!(ctx, "O1INV", ["O1", "O1_B"], [
                    (mode mode),
                    (attr "OFF1", "#FF"),
                    (attr "OMUX", "OFF1"),
                    (pin "O1")
                ]);
                fuzz_enum!(ctx, "O2INV", ["O2", "O2_B"], [
                    (mode mode),
                    (attr "OFF2", "#FF"),
                    (attr "OMUX", "OFF2"),
                    (pin "O2")
                ]);
            }
            fuzz_enum!(ctx, "T1INV", ["T1", "T1_B"], [
                (mode mode),
                (attr "T_USED", "0"),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#OFF"),
                (attr "TMUX", "TFF1"),
                (attr "OFF1", "#OFF"),
                (attr "OFF2", "#OFF"),
                (attr "OMUX", "#OFF"),
                (pin "T1"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "T2INV", ["T2", "T2_B"], [
                (mode mode),
                (attr "T_USED", "0"),
                (attr "TFF1", "#OFF"),
                (attr "TFF2", "#FF"),
                (attr "TMUX", "TFF2"),
                (attr "OFF1", "#OFF"),
                (attr "OFF2", "#OFF"),
                (attr "OMUX", "#OFF"),
                (pin "T2"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "TMUX", ["T1", "T2", "TFF1", "TFF2", "TFFDDR"], [
                (mode mode),
                (attr "T1INV", "T1"),
                (attr "T2INV", "T2"),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#FF"),
                (attr "T_USED", "0"),
                (attr "OMUX", "#OFF"),
                (pin "T1"),
                (pin "T2"),
                (pin "T")
            ]);
            // hack to avoid dragging IOB into it.
            for val in ["O1", "O2", "OFF1", "OFF2", "OFFDDR"] {
                if !edev.grid.kind.is_spartan3ea() {
                    fuzz_one!(ctx, "OMUX", val, [
                        (mode mode),
                        (attr "O1INV", "O1"),
                        (attr "O2INV", "O2"),
                        (attr "OFF1", "#FF"),
                        (attr "OFF2", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "TSMUX", "1"),
                        (attr "TMUX", "T1"),
                        (attr "T1INV", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFFDELMUX", "1"),
                        (pin "O1"),
                        (pin "O2"),
                        (pin "T1"),
                        (pin "T"),
                        (pin "I")
                    ], [
                        (attr_diff "OMUX", "OFFDDR", val)
                    ]);
                } else if edev.grid.kind == GridKind::Spartan3E {
                    fuzz_one!(ctx, "OMUX", val, [
                        (mode mode),
                        (attr "O1INV", "O1"),
                        (attr "O2INV", "O2"),
                        (attr "OFF1", "#FF"),
                        (attr "OFF2", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "TSMUX", "1"),
                        (attr "TMUX", "T1"),
                        (attr "T1INV", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1_DDRMUX", "1"),
                        (attr "O2_DDRMUX", "1"),
                        (attr "IDDRIN_MUX", "2"),
                        (pin "O1"),
                        (pin "O2"),
                        (pin "T1"),
                        (pin "T"),
                        (pin "I")
                    ], [
                        (attr_diff "OMUX", "OFFDDR", val)
                    ]);
                    if bel_id.to_idx() != 2 {
                        let obel = BelId::from_idx(bel_id.to_idx() ^ 1);
                        fuzz_enum!(ctx, "O1_DDRMUX", ["0", "1"], [
                            (mode mode),
                            (bel_unused obel),
                            (attr "OFF1", "#FF"),
                            (attr "OFF2", "#FF"),
                            (attr "OMUX", "OFFDDR"),
                            (attr "TSMUX", "1"),
                            (attr "TFF1", "#FF"),
                            (attr "IFF1", "#FF"),
                            (attr "TMUX", "TFF1"),
                            (attr "IMUX", "0"),
                            (attr "O1INV", "#OFF"),
                            (pin "ODDRIN1"),
                            (pin "I")
                        ]);
                        fuzz_enum!(ctx, "O2_DDRMUX", ["0", "1"], [
                            (mode mode),
                            (bel_unused obel),
                            (attr "OFF1", "#FF"),
                            (attr "OFF2", "#FF"),
                            (attr "OMUX", "OFFDDR"),
                            (attr "TSMUX", "1"),
                            (attr "TFF1", "#FF"),
                            (attr "IFF1", "#FF"),
                            (attr "TMUX", "TFF1"),
                            (attr "IMUX", "0"),
                            (attr "O2INV", "#OFF"),
                            (pin "ODDRIN2"),
                            (pin "I")
                        ]);
                    }
                } else {
                    fuzz_one!(ctx, "OMUX", val, [
                        (mode mode),
                        (attr "O1INV", "O1"),
                        (attr "O2INV", "O2"),
                        (attr "OFF1", "#FF"),
                        (attr "OFF2", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "TSMUX", "1"),
                        (attr "TMUX", "T1"),
                        (attr "T1INV", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "1"),
                        (attr "O1_DDRMUX", "1"),
                        (attr "O2_DDRMUX", "1"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "SEL_MUX", "0"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (pin "O1"),
                        (pin "O2"),
                        (pin "T1"),
                        (pin "T"),
                        (pin "I")
                    ], [
                        (attr_diff "OMUX", "OFFDDR", val)
                    ]);
                    if bel_id.to_idx() != 2 {
                        let obel = BelId::from_idx(bel_id.to_idx() ^ 1);
                        fuzz_enum!(ctx, "O1_DDRMUX", ["0", "1"], [
                            (mode mode),
                            (bel_unused obel),
                            (attr "OFF1", "#FF"),
                            (attr "OFF2", "#FF"),
                            (attr "OMUX", "OFFDDR"),
                            (attr "TSMUX", "1"),
                            (attr "TFF1", "#FF"),
                            (attr "IFF1", "#FF"),
                            (attr "TMUX", "TFF1"),
                            (attr "IMUX", "0"),
                            (attr "O1INV", "#OFF"),
                            (attr "SEL_MUX", "0"),
                            (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                            (pin "ODDRIN1"),
                            (pin "I")
                        ]);
                        fuzz_enum!(ctx, "O2_DDRMUX", ["0", "1"], [
                            (mode mode),
                            (bel_unused obel),
                            (attr "OFF1", "#FF"),
                            (attr "OFF2", "#FF"),
                            (attr "OMUX", "OFFDDR"),
                            (attr "TSMUX", "1"),
                            (attr "TFF1", "#FF"),
                            (attr "IFF1", "#FF"),
                            (attr "TMUX", "TFF1"),
                            (attr "IMUX", "0"),
                            (attr "O2INV", "#OFF"),
                            (attr "SEL_MUX", "0"),
                            (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                            (pin "ODDRIN2"),
                            (pin "I")
                        ]);
                    }
                }
            }

            // Output flops
            if !edev.grid.kind.is_spartan3ea() {
                fuzz_enum!(ctx, "OFF1", ["#FF", "#LATCH"], [
                    (mode mode),
                    (attr "OFF2", "#OFF"),
                    (attr "OCEINV", "OCE_B"),
                    (attr "OFF1_INIT_ATTR", "INIT1"),
                    (pin "OCE")
                ]);
                fuzz_enum!(ctx, "OFF2", ["#FF", "#LATCH"], [
                    (mode mode),
                    (attr "OFF1", "#OFF"),
                    (attr "OCEINV", "OCE_B"),
                    (attr "OFF2_INIT_ATTR", "INIT1"),
                    (pin "OCE")
                ]);
            } else {
                fuzz_enum!(ctx, "OFF1", ["#FF", "#LATCH"], [
                    (mode mode),
                    (attr "OFF2", "#OFF"),
                    (attr "OCEINV", "OCE_B"),
                    (attr "PCICE_MUX", "OCE"),
                    (attr "OFF1_INIT_ATTR", "INIT1"),
                    (pin "OCE")
                ]);
                fuzz_enum!(ctx, "OFF2", ["#FF", "#LATCH"], [
                    (mode mode),
                    (attr "OFF1", "#OFF"),
                    (attr "OCEINV", "OCE_B"),
                    (attr "PCICE_MUX", "OCE"),
                    (attr "OFF2_INIT_ATTR", "INIT1"),
                    (pin "OCE")
                ]);
            }
            fuzz_enum!(ctx, "TFF1", ["#FF", "#LATCH"], [
                (mode mode),
                (attr "TFF2", "#OFF"),
                (attr "TCEINV", "TCE_B"),
                (attr "TFF1_INIT_ATTR", "INIT1"),
                (pin "TCE")
            ]);
            fuzz_enum!(ctx, "TFF2", ["#FF", "#LATCH"], [
                (mode mode),
                (attr "TFF1", "#OFF"),
                (attr "TCEINV", "TCE_B"),
                (attr "TFF2_INIT_ATTR", "INIT1"),
                (pin "TCE")
            ]);
            fuzz_enum!(ctx, "OFF1_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OFF1_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "OFF2_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "OFF2", "#FF"),
                (attr "OFF2_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "TFF1_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (attr "TFF1_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "TFF2_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "TFF2", "#FF"),
                (attr "TFF2_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "OFF1_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OFF2", "#FF"),
                (attr "OFF1_SR_ATTR", "SRHIGH"),
                (attr "OFF2_SR_ATTR", "SRHIGH"),
                (attr "OFF2_INIT_ATTR", "#OFF")
            ]);
            fuzz_enum!(ctx, "OFF2_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OFF2", "#FF"),
                (attr "OFF1_SR_ATTR", "SRHIGH"),
                (attr "OFF2_SR_ATTR", "SRHIGH"),
                (attr "OFF1_INIT_ATTR", "#OFF")
            ]);
            fuzz_enum!(ctx, "TFF1_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#FF"),
                (attr "TFF1_SR_ATTR", "SRHIGH"),
                (attr "TFF2_SR_ATTR", "SRHIGH"),
                (attr "TFF2_INIT_ATTR", "#OFF")
            ]);
            fuzz_enum!(ctx, "TFF2_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#FF"),
                (attr "TFF1_SR_ATTR", "SRHIGH"),
                (attr "TFF2_SR_ATTR", "SRHIGH"),
                (attr "TFF1_INIT_ATTR", "#OFF")
            ]);
            fuzz_enum!(ctx, "OFFATTRBOX", ["SYNC", "ASYNC"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OFF2", "#FF")
            ]);
            fuzz_enum!(ctx, "TFFATTRBOX", ["SYNC", "ASYNC"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#FF")
            ]);

            // Input flops
            fuzz_enum!(ctx, "IFF1", ["#FF", "#LATCH"], [
                (mode mode),
                (attr "IFF2", "#OFF"),
                (attr "ICEINV", "ICE_B"),
                (attr "IFF1_INIT_ATTR", "INIT1"),
                (pin "ICE")
            ]);
            fuzz_enum!(ctx, "IFF2", ["#FF", "#LATCH"], [
                (mode mode),
                (attr "IFF1", "#OFF"),
                (attr "ICEINV", "ICE_B"),
                (attr "IFF2_INIT_ATTR", "INIT1"),
                (pin "ICE")
            ]);
            fuzz_enum!(ctx, "IFF1_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "IFF1_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "IFF2_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "IFF2", "#FF"),
                (attr "IFF2_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "IFF1_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "IFF1_SR_ATTR", "SRHIGH")
            ]);
            fuzz_enum!(ctx, "IFF2_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "IFF2", "#FF"),
                (attr "IFF2_SR_ATTR", "SRHIGH")
            ]);
            fuzz_enum!(ctx, "IFFATTRBOX", ["SYNC", "ASYNC"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "IFF2", "#FF")
            ]);

            // Input path.
            if edev.grid.kind == GridKind::Spartan3E {
                fuzz_enum!(ctx, "IDDRIN_MUX", ["0", "1", "2"], [
                    (mode mode),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "1"),
                    (attr "IFFDMUX", "#OFF"),
                    (pin "IDDRIN1"),
                    (pin "IDDRIN2"),
                    (pin "I")
                ]);
            } else if edev.grid.kind.is_spartan3a() {
                fuzz_enum!(ctx, "IDDRIN_MUX", ["0", "1"], [
                    (mode mode),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "1"),
                    (attr "SEL_MUX", "0"),
                    (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (pin "IDDRIN1"),
                    (pin "IDDRIN2"),
                    (pin "I")
                ]);
                fuzz_one!(ctx, "IDDRIN_MUX", "2", [
                    (mode mode),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "1"),
                    (attr "SEL_MUX", "0"),
                    (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (pin "IDDRIN1"),
                    (pin "IDDRIN2"),
                    (pin "I")
                ], [
                    (attr "IDDRIN_MUX", "2"),
                    (attr "IFFDMUX", "1")
                ]);
            }

            if !edev.grid.kind.is_spartan3a() {
                if edev.grid.kind != GridKind::Spartan3E {
                    fuzz_enum!(ctx, "IDELMUX", ["0", "1"], [
                        (mode mode),
                        (attr "IFFDELMUX", "0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFFDELMUX", ["0", "1"], [
                        (mode mode),
                        (attr "IDELMUX", "0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                        (mode mode),
                        (attr "TSMUX", "1"),
                        (attr "IDELMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "0"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1INV", "O1"),
                        (attr "OMUX", "O1"),
                        (attr "T1INV", "T1"),
                        (attr "TMUX", "T1"),
                        (attr "T_USED", "0"),
                        (pin "O1"),
                        (pin "T1"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFFDMUX", ["0", "1"], [
                        (mode mode),
                        (attr "TSMUX", "1"),
                        (attr "IDELMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1INV", "O1"),
                        (attr "OMUX", "O1"),
                        (attr "T1INV", "T1"),
                        (attr "TMUX", "T1"),
                        (attr "T_USED", "0"),
                        (pin "O1"),
                        (pin "T1"),
                        (pin "I")
                    ]);
                } else {
                    fuzz_enum!(ctx, "IDELMUX", ["0", "1"], [
                        (mode mode),
                        (attr "IFFDELMUX", "0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "IBUF_DELAY_VALUE", "DLY4"),
                        (attr "PRE_DELAY_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFFDELMUX", ["0", "1"], [
                        (mode mode),
                        (attr "IDELMUX", "0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "IBUF_DELAY_VALUE", "DLY4"),
                        (attr "PRE_DELAY_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                        (mode mode),
                        (attr "TSMUX", "1"),
                        (attr "IDELMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "0"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1INV", "O1"),
                        (attr "OMUX", "O1"),
                        (attr "T1INV", "T1"),
                        (attr "TMUX", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IDDRIN_MUX", "2"),
                        (pin "O1"),
                        (pin "T1"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFFDMUX", ["0", "1"], [
                        (mode mode),
                        (attr "TSMUX", "1"),
                        (attr "IDELMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1INV", "O1"),
                        (attr "OMUX", "O1"),
                        (attr "T1INV", "T1"),
                        (attr "TMUX", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IDDRIN_MUX", "2"),
                        (pin "O1"),
                        (pin "T1"),
                        (pin "I")
                    ]);
                }
                fuzz_enum!(ctx, "TSMUX", ["0", "1"], [
                    (mode mode),
                    (attr "IFFDMUX", "1"),
                    (attr "TMUX", "T1"),
                    (attr "T1INV", "T1"),
                    (attr "OMUX", "O1"),
                    (attr "O1INV", "O1"),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "0"),
                    (attr "T_USED", "0"),
                    (pin "T1"),
                    (pin "O1"),
                    (pin "I"),
                    (pin "T")
                ]);
            } else {
                if name.ends_with("T") || name.ends_with("B") {
                    fuzz_enum!(ctx, "IBUF_DELAY_VALUE", ["DLY0", "DLY16"], [
                        (mode mode),
                        (attr "IFD_DELAY_VALUE", "DLY0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFD_DELAY_VALUE", ["DLY0", "DLY8"], [
                        (mode mode),
                        (attr "IBUF_DELAY_VALUE", "DLY0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ]);
                } else {
                    fuzz_enum!(ctx, "IBUF_DELAY_VALUE", [
                        "DLY0",
                        "DLY1",
                        "DLY2",
                        "DLY3",
                        "DLY4",
                        "DLY5",
                        "DLY6",
                        "DLY7",
                        "DLY8",
                        "DLY9",
                        "DLY10",
                        "DLY11",
                        "DLY12",
                        "DLY13",
                        "DLY14",
                        "DLY15",
                        "DLY16",
                    ], [
                        (mode mode),
                        (attr "IFD_DELAY_VALUE", "DLY0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFD_DELAY_VALUE", [
                        "DLY0",
                        "DLY1",
                        "DLY2",
                        "DLY3",
                        "DLY4",
                        "DLY5",
                        "DLY6",
                        "DLY7",
                        "DLY8",
                    ], [
                        (mode mode),
                        (attr "IBUF_DELAY_VALUE", "DLY0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_one!(ctx, "DELAY_ADJ_ATTRBOX", "VARIABLE", [
                        (mode mode),
                        (attr "IBUF_DELAY_VALUE", "DLY16"),
                        (attr "IFD_DELAY_VALUE", "DLY8"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ], [
                        (attr_diff "DELAY_ADJ_ATTRBOX", "FIXED", "VARIABLE")
                    ]);
                }
                fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                    (mode mode),
                    (attr "TSMUX", "1"),
                    (attr "IFF1", "#FF"),
                    (attr "IFFDMUX", "0"),
                    (attr "O1INV", "O1"),
                    (attr "OMUX", "O1"),
                    (attr "T1INV", "T1"),
                    (attr "TMUX", "T1"),
                    (attr "T_USED", "0"),
                    (attr "IDDRIN_MUX", "2"),
                    (attr "SEL_MUX", "0"),
                    (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (pin "O1"),
                    (pin "T1"),
                    (pin "I")
                ]);
                fuzz_enum!(ctx, "IFFDMUX", ["0", "1"], [
                    (mode mode),
                    (attr "TSMUX", "1"),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "0"),
                    (attr "O1INV", "O1"),
                    (attr "OMUX", "O1"),
                    (attr "T1INV", "T1"),
                    (attr "TMUX", "T1"),
                    (attr "T_USED", "0"),
                    (attr "IDDRIN_MUX", "2"),
                    (attr "SEL_MUX", "0"),
                    (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (pin "O1"),
                    (pin "T1"),
                    (pin "I")
                ]);
                fuzz_enum!(ctx, "TSMUX", ["0", "1"], [
                    (mode mode),
                    (attr "IFFDMUX", "1"),
                    (attr "TMUX", "T1"),
                    (attr "T1INV", "T1"),
                    (attr "OMUX", "O1"),
                    (attr "O1INV", "O1"),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "0"),
                    (attr "T_USED", "0"),
                    (attr "SEL_MUX", "0"),
                    (pin "T1"),
                    (pin "O1"),
                    (pin "I"),
                    (pin "T")
                ]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let intdb = ctx.edev.egrid().db;

    // IOI
    for (node_kind, tile, node) in &intdb.nodes {
        if !tile.starts_with("IOI") {
            continue;
        }
        if ctx.edev.egrid().node_index[node_kind].is_empty() {
            continue;
        }
        let int_tiles = &[match edev.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => match &tile[..] {
                "IOI.CLK_B" => "INT.IOI.CLK_B",
                "IOI.CLK_T" => "INT.IOI.CLK_T",
                _ => "INT.IOI",
            },
            GridKind::Spartan3 => "INT.IOI.S3",
            GridKind::Spartan3E => "INT.IOI.S3E",
            GridKind::Spartan3A | GridKind::Spartan3ADsp => match &tile[..] {
                "IOI.S3A.L" | "IOI.S3A.R" | "IOI.S3ADSP.L" | "IOI.S3ADSP.R" => "INT.IOI.S3A.LR",
                _ => "INT.IOI.S3A.TB",
            },
        }];

        for bel in node.bels.keys() {
            if !bel.starts_with("IOI") {
                continue;
            }
            ctx.collect_inv(tile, bel, "OTCLK1");
            ctx.collect_inv(tile, bel, "OTCLK2");
            ctx.collect_inv(tile, bel, "ICLK1");
            ctx.collect_inv(tile, bel, "ICLK2");
            ctx.collect_int_inv(int_tiles, tile, bel, "SR", edev.grid.kind.is_virtex2());
            ctx.collect_int_inv(int_tiles, tile, bel, "OCE", edev.grid.kind.is_virtex2());
            ctx.collect_inv(tile, bel, "REV");
            ctx.collect_inv(tile, bel, "ICE");
            ctx.collect_inv(tile, bel, "TCE");
            let item = ctx.extract_bit(tile, bel, "ISR_USED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_SR_EN", item);
            let item = ctx.extract_bit(tile, bel, "OSR_USED", "0");
            ctx.tiledb.insert(tile, bel, "OFF_SR_EN", item);
            let item = ctx.extract_bit(tile, bel, "TSR_USED", "0");
            ctx.tiledb.insert(tile, bel, "TFF_SR_EN", item);
            let item = ctx.extract_bit(tile, bel, "IREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_REV_EN", item);
            let item = ctx.extract_bit(tile, bel, "OREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "OFF_REV_EN", item);
            let item = ctx.extract_bit(tile, bel, "TREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "TFF_REV_EN", item);

            if edev.grid.kind.is_spartan3ea() {
                ctx.collect_enum_default(tile, bel, "PCICE_MUX", &["OCE", "PCICE"], "NONE");
            }
            ctx.collect_inv(tile, bel, "O1");
            ctx.collect_inv(tile, bel, "O2");
            ctx.collect_inv(tile, bel, "T1");
            ctx.collect_inv(tile, bel, "T2");
            ctx.collect_enum_default(
                tile,
                bel,
                "TMUX",
                &["T1", "T2", "TFF1", "TFF2", "TFFDDR"],
                "NONE",
            );
            // hack to avoid dragging IOB into it.
            let mut item = xlat_enum(vec![
                (
                    "O1".to_string(),
                    ctx.state.get_diff(tile, bel, "OMUX", "O1"),
                ),
                (
                    "O2".to_string(),
                    ctx.state.get_diff(tile, bel, "OMUX", "O2"),
                ),
                (
                    "OFF1".to_string(),
                    ctx.state.get_diff(tile, bel, "OMUX", "OFF1"),
                ),
                (
                    "OFF2".to_string(),
                    ctx.state.get_diff(tile, bel, "OMUX", "OFF2"),
                ),
                (
                    "OFFDDR".to_string(),
                    ctx.state.get_diff(tile, bel, "OMUX", "OFFDDR"),
                ),
            ]);
            let TileItemKind::Enum { ref mut values } = item.kind else {
                unreachable!()
            };
            values.insert("NONE".into(), BitVec::repeat(false, item.bits.len()));
            ctx.tiledb.insert(tile, bel, "OMUX", item);

            let item = ctx.extract_enum_bool(tile, bel, "IFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF1_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF2_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF1_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF2_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "IFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "IFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "OFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "OFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "TFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "TFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "IFF1_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "IFF2_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "IFF_SYNC", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "OFF_SYNC", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "TFF_SYNC", item);

            // Input path
            ctx.tiledb.insert(
                tile,
                bel,
                "TSBYPASS_MUX",
                xlat_enum(vec![
                    (
                        "GND".to_string(),
                        ctx.state.get_diff(tile, bel, "TSMUX", "0"),
                    ),
                    (
                        "TMUX".to_string(),
                        ctx.state.get_diff(tile, bel, "TSMUX", "1"),
                    ),
                ]),
            );

            if !edev.grid.kind.is_spartan3a() {
                let item = ctx.extract_enum_bool(tile, bel, "IDELMUX", "1", "0");
                ctx.tiledb.insert(tile, bel, "I_DELAY_EN", item);
                let item = ctx.extract_enum_bool(tile, bel, "IFFDELMUX", "1", "0");
                ctx.tiledb.insert(tile, bel, "IFF_DELAY_EN", item);
            } else {
                let item_i = ctx.extract_enum_bool(tile, bel, "IBUF_DELAY_VALUE", "DLY0", "DLY16");
                let item_iff = ctx.extract_enum_bool(tile, bel, "IFD_DELAY_VALUE", "DLY0", "DLY8");
                if tile.ends_with("L") || tile.ends_with("R") {
                    let en_i = Diff::from_bool_item(&item_i);
                    let en_iff = Diff::from_bool_item(&item_iff);
                    let common = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8")
                        .combine(&!&en_i);
                    assert_eq!(
                        common,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4")
                            .combine(&!&en_iff)
                    );
                    // I
                    let b0_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY15")
                        .combine(&!&en_i);
                    let b1_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY14")
                        .combine(&!&en_i);
                    let b2_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12")
                        .combine(&!&en_i);
                    for (val, diffs) in [
                        ("DLY13", &[&b0_i, &b1_i][..]),
                        ("DLY11", &[&b0_i, &b2_i][..]),
                        ("DLY10", &[&b1_i, &b2_i][..]),
                        ("DLY9", &[&b0_i, &b1_i, &b2_i][..]),
                        ("DLY7", &[&b0_i, &common][..]),
                        ("DLY6", &[&b1_i, &common][..]),
                        ("DLY5", &[&b0_i, &b1_i, &common][..]),
                        ("DLY4", &[&b2_i, &common][..]),
                        ("DLY3", &[&b0_i, &b2_i, &common][..]),
                        ("DLY2", &[&b1_i, &b2_i, &common][..]),
                        ("DLY1", &[&b0_i, &b1_i, &b2_i, &common][..]),
                    ] {
                        let mut diff = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", val);
                        for &d in diffs {
                            diff = diff.combine(&!d);
                        }
                        diff = diff.combine(&!&en_i);
                        diff.assert_empty();
                    }
                    ctx.tiledb
                        .insert(tile, bel, "I_DELAY", xlat_bitvec(vec![!b0_i, !b1_i, !b2_i]));

                    // IFF
                    let b0_iff = ctx
                        .state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY7")
                        .combine(&!&en_iff);
                    let b1_iff = ctx
                        .state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6")
                        .combine(&!&en_iff);
                    for (val, diffs) in [
                        ("DLY5", &[&b0_iff, &b1_iff][..]),
                        ("DLY3", &[&b0_iff, &common][..]),
                        ("DLY2", &[&b1_iff, &common][..]),
                        ("DLY1", &[&b0_iff, &b1_iff, &common][..]),
                    ] {
                        let mut diff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", val);
                        for &d in diffs {
                            diff = diff.combine(&!d);
                        }
                        diff = diff.combine(&!&en_iff);
                        diff.assert_empty();
                    }
                    ctx.tiledb
                        .insert(tile, bel, "IFF_DELAY", xlat_bitvec(vec![!b0_iff, !b1_iff]));
                    ctx.tiledb
                        .insert(tile, bel, "DELAY_COMMON", xlat_bitvec(vec![!common]));
                    let item = ctx.extract_bit(tile, bel, "DELAY_ADJ_ATTRBOX", "VARIABLE");
                    ctx.tiledb.insert(tile, bel, "DELAY_VARIABLE", item);
                }
                ctx.tiledb.insert(tile, bel, "I_DELAY_EN", item_i);
                ctx.tiledb.insert(tile, bel, "IFF_DELAY_EN", item_iff);
            }
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "I_TSBYPASS_EN", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFDMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "IFF_TSBYPASS_EN", item);

            if edev.grid.kind.is_spartan3ea() {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "IDDRIN_MUX",
                    xlat_enum_default(
                        vec![
                            (
                                "IFFDMUX".to_string(),
                                ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "2"),
                            ),
                            (
                                "IDDRIN1".to_string(),
                                ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "1"),
                            ),
                            (
                                "IDDRIN2".to_string(),
                                ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "0"),
                            ),
                        ],
                        "NONE",
                    ),
                );
            }
        }
        // specials. need cross-bel discard.
        if edev.grid.kind.is_spartan3ea() {
            for bel in ["IOI0", "IOI1"] {
                let obel = if bel == "IOI0" { "IOI1" } else { "IOI0" };
                ctx.state
                    .get_diff(tile, bel, "O1_DDRMUX", "1")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, bel, "O2_DDRMUX", "1")
                    .assert_empty();
                let mut diff = ctx.state.get_diff(tile, bel, "O1_DDRMUX", "0");
                diff.discard_bits(ctx.tiledb.item(tile, obel, "OMUX"));
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "O1_DDRMUX",
                    xlat_enum(vec![
                        ("O1".to_string(), Diff::default()),
                        ("ODDRIN1".to_string(), diff),
                    ]),
                );
                let mut diff = ctx.state.get_diff(tile, bel, "O2_DDRMUX", "0");
                diff.discard_bits(ctx.tiledb.item(tile, obel, "OMUX"));
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "O2_DDRMUX",
                    xlat_enum(vec![
                        ("O2".to_string(), Diff::default()),
                        ("ODDRIN2".to_string(), diff),
                    ]),
                );
            }
        }
    }
}
