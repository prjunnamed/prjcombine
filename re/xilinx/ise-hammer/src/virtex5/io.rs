use bitvec::prelude::*;
use prjcombine_interconnect::grid::{DieId, NodeLoc, RowId, TileIobId};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, extract_bitvec_val,
    extract_bitvec_val_part, xlat_bit, xlat_bit_wide, xlat_bitvec, xlat_enum, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_virtex4::{bels, expanded::IoCoord};
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        iostd::{DciKind, DiffKind, Iostd},
        props::{
            DynProp,
            bel::BaseBelMode,
            relation::{FixedRelation, NodeRelation, Related},
        },
    },
    virtex4::io::IsBonded,
};

const IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6", "8"]),
    Iostd::cmos("PCI33_3", 3300, &[]),
    Iostd::cmos("PCI66_3", 3300, &[]),
    Iostd::cmos("PCIX", 3300, &[]),
    Iostd::odci("LVDCI_33", 3300),
    Iostd::odci("LVDCI_25", 2500),
    Iostd::odci("LVDCI_18", 1800),
    Iostd::odci("LVDCI_15", 1500),
    Iostd::odci_half("LVDCI_DV2_25", 2500),
    Iostd::odci_half("LVDCI_DV2_18", 1800),
    Iostd::odci_half("LVDCI_DV2_15", 1500),
    Iostd::odci_vref("HSLVDCI_33", 3300, 1650),
    Iostd::odci_vref("HSLVDCI_25", 2500, 1250),
    Iostd::odci_vref("HSLVDCI_18", 1800, 900),
    Iostd::odci_vref("HSLVDCI_15", 1500, 750),
    Iostd::vref_od("GTL", 800),
    Iostd::vref_od("GTLP", 1000),
    Iostd::vref("SSTL2_I", 2500, 1250),
    Iostd::vref("SSTL2_II", 2500, 1250),
    Iostd::vref("SSTL18_I", 1800, 900),
    Iostd::vref("SSTL18_II", 1800, 900),
    Iostd::vref("HSTL_I_18", 1800, 900),
    Iostd::vref("HSTL_II_18", 1800, 900),
    Iostd::vref("HSTL_III_18", 1800, 1080),
    Iostd::vref("HSTL_IV_18", 1800, 1080),
    Iostd::vref("HSTL_I", 1500, 750),
    Iostd::vref("HSTL_II", 1500, 750),
    Iostd::vref("HSTL_III", 1500, 900),
    Iostd::vref("HSTL_IV", 1500, 900),
    Iostd::vref("HSTL_I_12", 1200, 600),
    Iostd::vref_dci_od("GTL_DCI", 1200, 800),
    Iostd::vref_dci_od("GTLP_DCI", 1500, 1000),
    Iostd::vref_dci("SSTL2_I_DCI", 2500, 1250, DciKind::InputSplit),
    Iostd::vref_dci("SSTL2_II_DCI", 2500, 1250, DciKind::BiSplit),
    Iostd::vref_dci("SSTL2_II_T_DCI", 2500, 1250, DciKind::BiSplitT),
    Iostd::vref_dci("SSTL18_I_DCI", 1800, 900, DciKind::InputSplit),
    Iostd::vref_dci("SSTL18_II_DCI", 1800, 900, DciKind::BiSplit),
    Iostd::vref_dci("SSTL18_II_T_DCI", 1800, 900, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_I_DCI_18", 1800, 900, DciKind::InputSplit),
    Iostd::vref_dci("HSTL_II_DCI_18", 1800, 900, DciKind::BiSplit),
    Iostd::vref_dci("HSTL_II_T_DCI_18", 1800, 900, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_III_DCI_18", 1800, 1080, DciKind::InputVcc),
    Iostd::vref_dci("HSTL_IV_DCI_18", 1800, 1080, DciKind::BiVcc),
    Iostd::vref_dci("HSTL_I_DCI", 1500, 750, DciKind::InputSplit),
    Iostd::vref_dci("HSTL_II_DCI", 1500, 750, DciKind::BiSplit),
    Iostd::vref_dci("HSTL_II_T_DCI", 1500, 750, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_III_DCI", 1500, 900, DciKind::InputVcc),
    Iostd::vref_dci("HSTL_IV_DCI", 1500, 900, DciKind::BiVcc),
    Iostd::pseudo_diff("DIFF_SSTL2_I", 2500),
    Iostd::pseudo_diff("DIFF_SSTL2_II", 2500),
    Iostd::pseudo_diff("DIFF_SSTL18_I", 1800),
    Iostd::pseudo_diff("DIFF_SSTL18_II", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_I", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_II", 1500),
    Iostd::pseudo_diff("BLVDS_25", 2500),
    Iostd::pseudo_diff("LVPECL_25", 2500),
    Iostd::pseudo_diff_dci("DIFF_SSTL2_I_DCI", 2500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL2_II_DCI", 2500, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_I_DCI", 1800, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_II_DCI", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_I_DCI_18", 1800, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI_18", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_I_DCI", 1500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI", 1500, DciKind::BiSplit),
    Iostd::true_diff("LVDS_25", 2500),
    Iostd::true_diff("LVDSEXT_25", 2500),
    Iostd::true_diff("RSDS_25", 2500),
    Iostd::true_diff("HT_25", 2500),
];

#[derive(Copy, Clone, Debug)]
pub struct HclkIoi;

impl NodeRelation for HclkIoi {
    fn resolve(&self, backend: &IseBackend, nloc: NodeLoc) -> Option<NodeLoc> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[nloc.0];
        let row = chip.row_hclk(nloc.2);
        Some(
            edev.egrid
                .get_node_by_bel((nloc.0, (nloc.1, row), bels::IDELAYCTRL)),
        )
    }
}

fn get_vrefs(backend: &IseBackend, nloc: NodeLoc) -> Vec<NodeLoc> {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let (die, col, row, _) = nloc;

    let reg = edev.chips[die].row_to_reg(row);
    let bot = edev.chips[die].row_reg_bot(reg);
    let rows = if col == edev.col_cfg
        && (reg == edev.chips[die].reg_cfg || reg == edev.chips[die].reg_cfg - 2)
    {
        vec![bot + 15]
    } else if col == edev.col_cfg
        && (reg == edev.chips[die].reg_cfg - 1 || reg == edev.chips[die].reg_cfg + 1)
    {
        vec![bot + 5]
    } else {
        vec![bot + 5, bot + 15]
    };
    rows.into_iter()
        .map(|vref_row| {
            edev.egrid
                .get_node_by_bel((die, (col, vref_row), bels::IOB0))
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Vref;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Vref {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };

        let vrefs = get_vrefs(backend, nloc);
        if vrefs.contains(&nloc) {
            return None;
        }
        let chip = edev.chips[nloc.0];

        let hclk_row = chip.row_hclk(nloc.2);
        // Take exclusive mutex on VREF.
        let hclk_ioi =
            backend
                .egrid
                .get_node_by_bel((nloc.0, (nloc.1, hclk_row), bels::IDELAYCTRL));
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "VREF".to_string()),
            None,
            "EXCLUSIVE",
        );
        for vref in vrefs {
            let site = backend
                .ngrid
                .get_bel_name((vref.0, (vref.1, vref.2), bels::IOB0))
                .unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
            fuzzer.info.features.push(FuzzerFeature {
                id: FeatureId {
                    tile: "IO".into(),
                    bel: "IOB0".into(),
                    attr: "PRESENT".into(),
                    val: "VREF".into(),
                },
                tiles: backend.edev.node_bits(vref),
            });
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VrefInternal(pub &'static str, pub u32);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VrefInternal {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[nloc.0];
        let hclk_row = chip.row_hclk(nloc.2);
        // Take exclusive mutex on VREF.
        let hclk_ioi = edev
            .egrid
            .find_node_by_kind(nloc.0, (nloc.1, hclk_row), |kind| kind == self.0)?;
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "VREF".to_string()),
            None,
            "EXCLUSIVE",
        );
        let io = edev.get_io_info(IoCoord {
            die: nloc.0,
            col: nloc.1,
            row: nloc.2,
            iob: TileIobId::from_idx(0),
        });
        fuzzer = fuzzer.fuzz(Key::InternalVref(io.bank), None, self.1);
        fuzzer.info.features.push(FuzzerFeature {
            id: FeatureId {
                tile: self.0.into(),
                bel: "INTERNAL_VREF".into(),
                attr: "VREF".into(),
                val: self.1.to_string(),
            },
            tiles: edev.node_bits(hclk_ioi),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
struct Dci(Option<&'static str>);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Dci {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[nloc.0];

        if nloc.1 == edev.col_cfg {
            // Center column is more trouble than it's worth.
            return None;
        }
        if nloc.2.to_idx() % 20 == 7 {
            // Not in VR tile please.
            return None;
        }
        // Ensure nothing is placed in VR.
        let vr_row = RowId::from_idx(nloc.2.to_idx() / 20 * 20 + 7);
        let node_vr = edev
            .egrid
            .get_node_by_kind(nloc.0, (nloc.1, vr_row), |kind| kind == "IO");
        for bel in [bels::IOB0, bels::IOB1] {
            let site = backend
                .ngrid
                .get_bel_name((nloc.0, (nloc.1, vr_row), bel))
                .unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Test VR.
        if self.0.is_some() {
            fuzzer.info.features.push(FuzzerFeature {
                id: FeatureId {
                    tile: "IO".into(),
                    bel: "IOB_COMMON".into(),
                    attr: "PRESENT".into(),
                    val: "VR".into(),
                },
                tiles: edev.node_bits(node_vr),
            });
        }
        // Take exclusive mutex on bank DCI.
        let hclk_ioi =
            edev.egrid
                .get_node_by_kind(nloc.0, (nloc.1, chip.row_hclk(vr_row)), |kind| {
                    kind == "HCLK_IOI"
                });
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "BANK_DCI".to_string()),
            None,
            "EXCLUSIVE",
        );
        // Test bank DCI.
        if let Some(std) = self.0 {
            fuzzer.info.features.push(FuzzerFeature {
                id: FeatureId {
                    tile: "HCLK_IOI".into(),
                    bel: "DCI".into(),
                    attr: "STD".into(),
                    val: std.into(),
                },
                tiles: edev.node_bits(hclk_ioi),
            });
        }
        // Take shared mutex on global DCI.
        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
        // Anchor global DCI by putting something in bottom IOB of center column.
        let iob_center = (nloc.0, (edev.col_cfg, chip.row_bufg() - 30), bels::IOB0);
        let site = backend.ngrid.get_bel_name(iob_center).unwrap();
        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_33");
        // Ensure anchor VR IOBs are free.
        for bel in [bels::IOB0, bels::IOB1] {
            let iob_center_vr = (nloc.0, (edev.col_cfg, chip.row_bufg() - 30 + 2), bel);
            let site = backend.ngrid.get_bel_name(iob_center_vr).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DiffOut(pub &'static str, pub &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DiffOut {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[nloc.0];
        let lvds_row = chip.row_hclk(nloc.2);
        // Take exclusive mutex on bank LVDS.
        let hclk_ioi = edev
            .egrid
            .get_node_by_bel((nloc.0, (nloc.1, lvds_row), bels::IDELAYCTRL));
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "BANK_LVDS".to_string()),
            None,
            "EXCLUSIVE",
        );

        let hclk_ioi_node = edev.egrid.node(hclk_ioi);
        fuzzer.info.features.push(FuzzerFeature {
            id: FeatureId {
                tile: edev.egrid.db.nodes.key(hclk_ioi_node.kind).clone(),
                bel: "LVDS".into(),
                attr: self.0.into(),
                val: self.1.into(),
            },
            tiles: edev.node_bits(hclk_ioi),
        });
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    let mut ctx = FuzzCtx::new(session, backend, "IO");

    if devdata_only {
        for i in 0..2 {
            let bel_other = bels::IODELAY[i ^ 1];
            let mut bctx = ctx.bel(bels::IODELAY[i]);
            bctx.mode("IODELAY")
                .global("LEGIDELAY", "ENABLE")
                .bel_mode(bel_other, "IODELAY")
                .bel_attr(bel_other, "IDELAY_VALUE", "")
                .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
                .prop(Related::new(
                    HclkIoi,
                    BaseBelMode::new(bels::IDELAYCTRL, "IDELAYCTRL".into()),
                ))
                .test_enum("IDELAY_TYPE", &["DEFAULT"]);
        }
        return;
    }

    let package = backend
        .device
        .bonds
        .values()
        .max_by_key(|bond| {
            let bdata = &backend.db.bonds[bond.bond];
            let prjcombine_re_xilinx_geom::Bond::Virtex4(bdata) = bdata else {
                unreachable!();
            };
            bdata.pins.len()
        })
        .unwrap();

    {
        let mut bctx = ctx.bel(bels::IOI);
        for i in 0..2 {
            for j in 0..2 {
                bctx.build()
                    .mutex(format!("MUX.ICLK{i}"), format!("CKINT{j}"))
                    .test_manual(format!("MUX.ICLK{i}"), format!("CKINT{j}"))
                    .pip(format!("ICLK{i}"), format!("CKINT{j}"))
                    .commit();
            }
            for j in 0..4 {
                bctx.build()
                    .mutex(format!("MUX.ICLK{i}"), format!("IOCLK{j}"))
                    .test_manual(format!("MUX.ICLK{i}"), format!("IOCLK{j}"))
                    .pip(format!("ICLK{i}"), format!("IOCLK{j}"))
                    .commit();
                bctx.build()
                    .mutex(format!("MUX.ICLK{i}"), format!("RCLK{j}"))
                    .test_manual(format!("MUX.ICLK{i}"), format!("RCLK{j}"))
                    .pip(format!("ICLK{i}"), format!("RCLK{j}"))
                    .commit();
            }
            for j in 0..10 {
                bctx.build()
                    .mutex(format!("MUX.ICLK{i}"), format!("HCLK{j}"))
                    .test_manual(format!("MUX.ICLK{i}"), format!("HCLK{j}"))
                    .pip(format!("ICLK{i}"), format!("HCLK{j}"))
                    .commit();
            }
        }
    }

    for i in 0..2 {
        let mut bctx = ctx.bel(bels::ILOGIC[i]);
        let bel_ologic = bels::OLOGIC[i];
        let bel_iodelay = bels::IODELAY[i];
        bctx.build()
            .bel_unused(bel_ologic)
            .test_manual("PRESENT", "ILOGIC")
            .mode("ILOGIC")
            .commit();
        bctx.build()
            .bel_unused(bel_ologic)
            .test_manual("PRESENT", "ISERDES")
            .mode("ISERDES")
            .commit();

        for (pin, pin_t, pin_c) in [("CLK", "CLK", "CLK_B"), ("CLKB", "CLKB_B", "CLKB")] {
            for j in 0..2 {
                bctx.build()
                    .tile_mutex("ICLK", "MUX")
                    .mutex(format!("MUX.{pin}"), format!("ICLK{j}"))
                    .test_manual(format!("MUX.{pin}"), format!("ICLK{j}"))
                    .pip(pin, (bels::IOI, format!("ICLK{j}")))
                    .commit();
                bctx.mode("ISERDES")
                    .tile_mutex("ICLK", format!("INV.{pin}.{i}.{j}"))
                    .pip(pin, (bels::IOI, format!("ICLK{j}")))
                    .pin(pin)
                    .test_manual(format!("INV.ICLK{j}"), "0")
                    .attr(format!("{pin}INV"), pin_t)
                    .commit();
                bctx.mode("ISERDES")
                    .tile_mutex("ICLK", format!("INV.{pin}.{i}.{j}"))
                    .pip(pin, (bels::IOI, format!("ICLK{j}")))
                    .pin(pin)
                    .test_manual(format!("INV.ICLK{j}"), "1")
                    .attr(format!("{pin}INV"), pin_c)
                    .commit();
            }

            bctx.mode("ISERDES")
                .bel_unused(bel_iodelay)
                .test_inv("CLKDIV");

            bctx.mode("ISERDES")
                .attr("INTERFACE_TYPE", "MEMORY")
                .attr("DATA_RATE", "SDR")
                .pin("OCLK")
                .test_enum_suffix("OCLKINV", "SDR", &["OCLK", "OCLK_B"]);
            bctx.mode("ISERDES")
                .attr("INTERFACE_TYPE", "MEMORY")
                .attr("DATA_RATE", "DDR")
                .pin("OCLK")
                .test_enum_suffix("OCLKINV", "DDR", &["OCLK", "OCLK_B"]);

            bctx.mode("ILOGIC")
                .attr("IFFTYPE", "#FF")
                .pin("SR")
                .test_enum("SRUSED", &["0"]);
            bctx.mode("ILOGIC")
                .attr("IFFTYPE", "#FF")
                .pin("REV")
                .test_enum("REVUSED", &["0"]);

            bctx.mode("ISERDES")
                .attr("DATA_WIDTH", "2")
                .test_enum("SERDES", &["FALSE", "TRUE"]);
            bctx.mode("ISERDES")
                .test_enum("SERDES_MODE", &["MASTER", "SLAVE"]);
            bctx.mode("ISERDES")
                .test_enum("INTERFACE_TYPE", &["NETWORKING", "MEMORY"]);
            bctx.mode("ISERDES")
                .attr("SERDES", "FALSE")
                .test_enum("DATA_WIDTH", &["2", "3", "4", "5", "6", "7", "8", "10"]);
            bctx.mode("ISERDES")
                .attr("SRTYPE", "SYNC")
                .test_enum_suffix("BITSLIP_ENABLE", "SYNC", &["FALSE", "TRUE"]);
            bctx.mode("ISERDES")
                .attr("SRTYPE", "ASYNC")
                .test_enum_suffix("BITSLIP_ENABLE", "ASYNC", &["FALSE", "TRUE"]);
            bctx.mode("ISERDES").test_enum("NUM_CE", &["1", "2"]);
            bctx.mode("ISERDES")
                .attr("INIT_BITSLIPCNT", "1111")
                .attr("INIT_RANK1_PARTIAL", "11111")
                .attr("INIT_RANK2", "111111")
                .attr("INIT_RANK3", "111111")
                .attr("INIT_CE", "11")
                .test_enum("DATA_RATE", &["SDR", "DDR"]);
            bctx.mode("ISERDES").test_enum(
                "DDR_CLK_EDGE",
                &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
            );

            bctx.mode("ILOGIC").attr("IFFTYPE", "DDR").test_enum(
                "DDR_CLK_EDGE",
                &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
            );
            bctx.mode("ILOGIC")
                .test_enum("IFFTYPE", &["#FF", "#LATCH", "DDR"]);
            for attr in [
                "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
                "SRVAL_Q4",
            ] {
                bctx.mode("ISERDES").test_enum(attr, &["0", "1"]);
            }

            bctx.mode("ILOGIC")
                .attr("IFFTYPE", "#FF")
                .test_enum("SRTYPE", &["SYNC", "ASYNC"]);
            bctx.mode("ISERDES").test_enum("SRTYPE", &["SYNC", "ASYNC"]);
            bctx.mode("ISERDES")
                .attr("DATA_RATE", "SDR")
                .test_multi_attr_bin("INIT_CE", 2);
            bctx.mode("ISERDES")
                .attr("DATA_RATE", "SDR")
                .test_multi_attr_bin("INIT_BITSLIPCNT", 4);
            bctx.mode("ISERDES")
                .attr("DATA_RATE", "SDR")
                .test_multi_attr_bin("INIT_RANK1_PARTIAL", 5);
            bctx.mode("ISERDES")
                .attr("DATA_RATE", "SDR")
                .test_multi_attr_bin("INIT_RANK2", 6);
            bctx.mode("ISERDES")
                .attr("DATA_RATE", "SDR")
                .test_multi_attr_bin("INIT_RANK3", 6);

            bctx.mode("ISERDES")
                .pin("OFB")
                .test_enum("OFB_USED", &["FALSE", "TRUE"]);
            bctx.mode("ISERDES")
                .pin("TFB")
                .test_enum("TFB_USED", &["FALSE", "TRUE"]);
            bctx.mode("ISERDES")
                .test_enum("IOBDELAY", &["NONE", "IFD", "IBUF", "BOTH"]);

            bctx.mode("ILOGIC")
                .attr("IMUX", "0")
                .attr("IDELMUX", "1")
                .attr("IFFMUX", "#OFF")
                .pin("D")
                .pin("DDLY")
                .pin("TFB")
                .pin("OFB")
                .pin("O")
                .test_enum("D2OBYP_SEL", &["GND", "T"]);
            bctx.mode("ILOGIC")
                .attr("IFFMUX", "0")
                .attr("IFFTYPE", "#FF")
                .attr("IFFDELMUX", "1")
                .attr("IMUX", "#OFF")
                .pin("D")
                .pin("DDLY")
                .pin("TFB")
                .pin("OFB")
                .test_enum("D2OFFBYP_SEL", &["GND", "T"]);
            bctx.mode("ILOGIC")
                .attr("IDELMUX", "1")
                .pin("D")
                .pin("DDLY")
                .pin("O")
                .pin("TFB")
                .pin("OFB")
                .test_enum("IMUX", &["0", "1"]);
            bctx.mode("ILOGIC")
                .attr("IFFDELMUX", "1")
                .attr("IFFTYPE", "#FF")
                .pin("D")
                .pin("DDLY")
                .pin("TFB")
                .pin("OFB")
                .test_enum("IFFMUX", &["0", "1"]);
            bctx.mode("ILOGIC")
                .attr("IMUX", "1")
                .attr("IFFMUX", "1")
                .attr("IFFTYPE", "#FF")
                .attr("IFFDELMUX", "0")
                .pin("D")
                .pin("DDLY")
                .pin("O")
                .pin("Q1")
                .pin("TFB")
                .pin("OFB")
                .test_enum("IDELMUX", &["0", "1"]);
            bctx.mode("ILOGIC")
                .attr("IMUX", "1")
                .attr("IFFMUX", "0")
                .attr("IFFTYPE", "#FF")
                .attr("IDELMUX", "0")
                .attr("D2OFFBYP_SEL", "T")
                .pin("D")
                .pin("DDLY")
                .pin("O")
                .pin("Q1")
                .pin("TFB")
                .pin("OFB")
                .test_enum("IFFDELMUX", &["0", "1"]);
        }
    }

    for i in 0..2 {
        let mut bctx = ctx.bel(bels::OLOGIC[i]);
        let bel_ilogic = bels::ILOGIC[i];
        bctx.build()
            .bel_unused(bel_ilogic)
            .test_manual("PRESENT", "OLOGIC")
            .mode("OLOGIC")
            .commit();
        bctx.build()
            .bel_unused(bel_ilogic)
            .test_manual("PRESENT", "OSERDES")
            .mode("OSERDES")
            .commit();

        bctx.mode("OLOGIC")
            .attr("ODDR_CLK_EDGE", "SAME_EDGE")
            .attr("OUTFFTYPE", "#FF")
            .attr("OMUX", "OUTFF")
            .pin("CLK")
            .pin("OQ")
            .test_enum_suffix("CLKINV", "SAME", &["CLK", "CLK_B"]);
        bctx.mode("OLOGIC")
            .attr("ODDR_CLK_EDGE", "OPPOSITE_EDGE")
            .attr("OUTFFTYPE", "#FF")
            .attr("OMUX", "OUTFF")
            .pin("CLK")
            .pin("OQ")
            .test_enum_suffix("CLKINV", "OPPOSITE", &["CLK", "CLK_B"]);
        bctx.mode("OLOGIC")
            .test_enum("ODDR_CLK_EDGE", &["SAME_EDGE", "OPPOSITE_EDGE"]);
        bctx.mode("OLOGIC")
            .test_enum("TDDR_CLK_EDGE", &["SAME_EDGE", "OPPOSITE_EDGE"]);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("DDR_CLK_EDGE", "SAME_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix("CLKINV", "SAME", &["CLK", "CLK_B"]);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("DDR_CLK_EDGE", "OPPOSITE_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix("CLKINV", "OPPOSITE", &["CLK", "CLK_B"]);
        bctx.mode("OSERDES")
            .test_enum("DDR_CLK_EDGE", &["SAME_EDGE", "OPPOSITE_EDGE"]);

        bctx.mode("OSERDES").test_inv("CLKDIV");

        for pin in ["D1", "D2", "D3", "D4", "D5", "D6"] {
            bctx.mode("OSERDES").test_inv(pin);
        }
        for pin in ["D1", "D2"] {
            bctx.mode("OLOGIC")
                .attr("OUTFFTYPE", "DDR")
                .attr("OMUX", "OUTFF")
                .pin("OQ")
                .test_inv(pin);
        }

        bctx.mode("OLOGIC")
            .attr("TMUX", "T1")
            .attr("T1USED", "0")
            .pin("TQ")
            .test_inv("T1");
        bctx.mode("OLOGIC")
            .attr("TFFTYPE", "DDR")
            .attr("TMUX", "TFF")
            .pin("TQ")
            .test_inv("T2");
        bctx.mode("OSERDES")
            .attr("DATA_RATE_TQ", "BUF")
            .test_inv("T1");
        for pin in ["T2", "T3", "T4"] {
            bctx.mode("OSERDES").test_inv(pin);
        }

        bctx.mode("OLOGIC")
            .attr("OUTFFTYPE", "#FF")
            .test_enum("SRTYPE_OQ", &["SYNC", "ASYNC"]);
        bctx.mode("OLOGIC")
            .attr("TFFTYPE", "#FF")
            .test_enum("SRTYPE_TQ", &["SYNC", "ASYNC"]);
        bctx.mode("OSERDES").test_enum("SRTYPE", &["SYNC", "ASYNC"]);

        bctx.mode("OLOGIC")
            .test_enum_suffix("INIT_OQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OLOGIC")
            .test_enum_suffix("INIT_TQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OSERDES")
            .test_enum_suffix("INIT_OQ", "OSERDES", &["0", "1"]);
        bctx.mode("OSERDES")
            .test_enum_suffix("INIT_TQ", "OSERDES", &["0", "1"]);

        bctx.mode("OLOGIC")
            .test_enum_suffix("SRVAL_OQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OLOGIC")
            .attr("TFFTYPE", "#FF")
            .attr("TMUX", "TFF")
            .pin("TQ")
            .test_enum_suffix("SRVAL_TQ", "FF", &["0", "1"]);
        bctx.mode("OLOGIC")
            .attr("TFFTYPE", "DDR")
            .attr("TMUX", "TFF")
            .pin("TQ")
            .test_enum_suffix("SRVAL_TQ", "DDR", &["0", "1"]);
        bctx.mode("OSERDES")
            .test_enum_suffix("SRVAL_OQ", "OSERDES", &["0", "1"]);
        bctx.mode("OSERDES")
            .test_enum_suffix("SRVAL_TQ", "OSERDES", &["0", "1"]);

        for attr in [
            "OSRUSED", "TSRUSED", "OREVUSED", "TREVUSED", "OCEUSED", "TCEUSED",
        ] {
            bctx.mode("OLOGIC")
                .attr("OUTFFTYPE", "#FF")
                .attr("TFFTYPE", "#FF")
                .pin("OCE")
                .pin("TCE")
                .pin("REV")
                .pin("SR")
                .test_enum(attr, &["0"]);
        }

        bctx.mode("OLOGIC")
            .pin("OQ")
            .test_enum("OUTFFTYPE", &["#FF", "#LATCH", "DDR"]);
        bctx.mode("OLOGIC")
            .pin("TQ")
            .test_enum("TFFTYPE", &["#FF", "#LATCH", "DDR"]);

        bctx.mode("OSERDES")
            .test_enum("DATA_RATE_OQ", &["SDR", "DDR"]);
        bctx.mode("OSERDES")
            .attr("T1INV", "T1")
            .pin("T1")
            .test_enum("DATA_RATE_TQ", &["BUF", "SDR", "DDR"]);

        bctx.mode("OLOGIC")
            .attr("OSRUSED", "#OFF")
            .attr("OREVUSED", "#OFF")
            .attr("OUTFFTYPE", "#FF")
            .attr("O1USED", "0")
            .attr("D1INV", "D1")
            .pin("D1")
            .pin("OQ")
            .test_enum("OMUX", &["D1", "OUTFF"]);
        bctx.mode("OLOGIC")
            .attr("TSRUSED", "#OFF")
            .attr("TREVUSED", "#OFF")
            .attr("TFFTYPE", "#FF")
            .attr("T1USED", "0")
            .attr("T1INV", "T1")
            .pin("T1")
            .pin("TQ")
            .test_enum("TMUX", &["T1", "TFF"]);

        bctx.mode("OLOGIC")
            .global("ENABLEMISR", "Y")
            .test_enum("MISR_ENABLE", &["FALSE", "TRUE"]);
        bctx.mode("OLOGIC")
            .global("ENABLEMISR", "Y")
            .test_enum("MISR_ENABLE_FDBK", &["FALSE", "TRUE"]);
        bctx.mode("OLOGIC")
            .global("ENABLEMISR", "Y")
            .test_enum("MISR_CLK_SELECT", &["CLK1", "CLK2"]);

        bctx.mode("OSERDES").test_enum("SERDES", &["FALSE", "TRUE"]);
        bctx.mode("OSERDES")
            .test_enum("SERDES_MODE", &["SLAVE", "MASTER"]);
        bctx.mode("OSERDES")
            .test_enum("TRISTATE_WIDTH", &["1", "4"]);
        bctx.mode("OSERDES")
            .test_enum("DATA_WIDTH", &["2", "3", "4", "5", "6", "7", "8", "10"]);
        bctx.mode("OSERDES").test_multi_attr_bin("INIT_LOADCNT", 4);

        bctx.build()
            .mutex("MUX.CLK", "CKINT")
            .test_manual("MUX.CLK", "CKINT")
            .pip("CLKMUX", "CKINT")
            .commit();
        bctx.build()
            .mutex("MUX.CLKDIV", "CKINT")
            .test_manual("MUX.CLKDIV", "CKINT")
            .pip("CLKDIVMUX", "CKINT_DIV")
            .commit();
        for i in 0..4 {
            bctx.build()
                .mutex("MUX.CLK", format!("IOCLK{i}"))
                .test_manual("MUX.CLK", format!("IOCLK{i}"))
                .pip("CLKMUX", (bels::IOI, format!("IOCLK{i}")))
                .commit();
            bctx.build()
                .mutex("MUX.CLK", format!("RCLK{i}"))
                .test_manual("MUX.CLK", format!("RCLK{i}"))
                .pip("CLKMUX", (bels::IOI, format!("RCLK{i}")))
                .commit();
            bctx.build()
                .mutex("MUX.CLKDIV", format!("RCLK{i}"))
                .test_manual("MUX.CLKDIV", format!("RCLK{i}"))
                .pip("CLKDIVMUX", (bels::IOI, format!("RCLK{i}")))
                .commit();
        }
        for i in 0..10 {
            bctx.build()
                .mutex("MUX.CLK", format!("HCLK{i}"))
                .test_manual("MUX.CLK", format!("HCLK{i}"))
                .pip("CLKMUX", (bels::IOI, format!("HCLK{i}")))
                .commit();
            bctx.build()
                .mutex("MUX.CLKDIV", format!("HCLK{i}"))
                .test_manual("MUX.CLKDIV", format!("HCLK{i}"))
                .pip("CLKDIVMUX", (bels::IOI, format!("HCLK{i}")))
                .commit();
        }
    }

    for i in 0..2 {
        let mut bctx = ctx.bel(bels::IODELAY[i]);
        let bel_ilogic = bels::ILOGIC[i];
        let bel_other = bels::IODELAY[i ^ 1];

        bctx.build()
            .bel_mode(bel_other, "IODELAY")
            .test_manual("PRESENT", "1")
            .mode("IODELAY")
            .commit();

        bctx.mode("IODELAY").bel_unused(bel_ilogic).test_inv("C");
        bctx.mode("IODELAY").test_inv("DATAIN");
        bctx.mode("IODELAY")
            .test_enum("HIGH_PERFORMANCE_MODE", &["FALSE", "TRUE"]);
        bctx.mode("IODELAY")
            .test_enum("DELAYCHAIN_OSC", &["FALSE", "TRUE"]);
        bctx.mode("IODELAY")
            .test_enum("DELAY_SRC", &["I", "O", "IO", "DATAIN"]);
        bctx.mode("IODELAY").test_multi_attr_dec("IDELAY_VALUE", 6);
        bctx.mode("IODELAY").test_multi_attr_dec("ODELAY_VALUE", 6);

        bctx.mode("IODELAY")
            .global("LEGIDELAY", "ENABLE")
            .bel_mode(bel_other, "IODELAY")
            .bel_attr(bel_other, "IDELAY_VALUE", "")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .prop(Related::new(
                HclkIoi,
                BaseBelMode::new(bels::IDELAYCTRL, "IDELAYCTRL".into()),
            ))
            .test_enum("IDELAY_TYPE", &["FIXED", "DEFAULT", "VARIABLE"]);
        bctx.mode("IODELAY")
            .global("LEGIDELAY", "DISABLE")
            .bel_mode(bel_other, "IODELAY")
            .bel_attr(bel_other, "IDELAY_VALUE", "")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .prop(Related::new(
                HclkIoi,
                BaseBelMode::new(bels::IDELAYCTRL, "IDELAYCTRL".into()),
            ))
            .test_manual("LEGIDELAY", "DISABLE")
            .attr("IDELAY_TYPE", "FIXED")
            .commit();
    }

    for i in 0..2 {
        let bel = bels::IOB[i];
        let mut bctx = ctx.bel(bel);
        let bel_ologic = bels::OLOGIC[i];
        let bel_iodelay = bels::IODELAY[i];
        let bel_other = bels::IOB[i ^ 1];
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_manual("PRESENT", "IOB")
            .mode("IOB")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "CONTINUOUS")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_manual("PRESENT", "IOB.CONTINUOUS")
            .mode("IOB")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_manual("PRESENT", "IPAD")
            .mode("IPAD")
            .commit();
        bctx.mode("IOB")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_enum("PULL", &["KEEPER", "PULLDOWN", "PULLUP"]);
        bctx.mode("IOB")
            .pin("O")
            .attr("OUSED", "0")
            .attr("OSTANDARD", "LVCMOS18")
            .test_multi_attr_bin("OPROGRAMMING", 31);
        bctx.mode("IOB")
            .attr("OUSED", "")
            .pin("I")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .attr("ISTANDARD", "LVCMOS18")
            .test_manual("IMUX", "I")
            .attr_diff("IMUX", "I_B", "I")
            .commit();
        for &std in IOSTDS {
            let mut vref_special = None;
            let mut dci_special = None;
            let mut dci_special_lite = None;
            if std.vref.is_some() {
                vref_special = Some(Vref);
            }
            if matches!(
                std.dci,
                DciKind::BiSplit
                    | DciKind::BiSplitT
                    | DciKind::BiVcc
                    | DciKind::InputSplit
                    | DciKind::InputVcc
            ) {
                dci_special = Some(Dci(Some(std.name)));
                dci_special_lite = Some(Dci(None));
            }
            if std.diff != DiffKind::None {
                bctx.mode(["IOBS", "IOBM"][i])
                    .attr("OUSED", "")
                    .pin("I")
                    .pin("DIFFI_IN")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .maybe_prop(dci_special)
                    .bel_mode(bel_other, ["IOBM", "IOBS"][i])
                    .bel_pin(bel_other, "PADOUT")
                    .bel_attr(bel_other, "OUSED", "")
                    .test_manual("ISTD", std.name)
                    .attr("IMUX", "I_B")
                    .attr("DIFFI_INUSED", "0")
                    .attr("ISTANDARD", std.name)
                    .attr("DIFF_TERM", "FALSE")
                    .bel_attr(bel_other, "PADOUTUSED", "0")
                    .bel_attr(bel_other, "ISTANDARD", std.name)
                    .bel_attr(bel_other, "DIFF_TERM", "FALSE")
                    .commit();
                if std.diff == DiffKind::True {
                    bctx.mode(["IOBS", "IOBM"][i])
                        .attr("OUSED", "")
                        .pin("I")
                        .pin("DIFFI_IN")
                        .attr("IMUX", "I_B")
                        .attr("DIFFI_INUSED", "0")
                        .attr("ISTANDARD", std.name)
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .maybe_prop(dci_special_lite)
                        .bel_mode(bel_other, ["IOBM", "IOBS"][i])
                        .bel_pin(bel_other, "PADOUT")
                        .bel_attr(bel_other, "OUSED", "")
                        .bel_attr(bel_other, "PADOUTUSED", "0")
                        .bel_attr(bel_other, "ISTANDARD", std.name)
                        .test_manual("DIFF_TERM", std.name)
                        .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                        .bel_attr_diff(bel_other, "DIFF_TERM", "FALSE", "TRUE")
                        .commit();
                }
            } else {
                bctx.mode("IOB")
                    .attr("OUSED", "")
                    .pin("I")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .maybe_prop(vref_special)
                    .maybe_prop(dci_special)
                    .test_manual("ISTD", std.name)
                    .attr("IMUX", "I_B")
                    .attr("ISTANDARD", std.name)
                    .commit();
            }
        }
        for &std in IOSTDS {
            if std.diff == DiffKind::True {
                if i == 1 {
                    bctx.build()
                        .attr("IMUX", "")
                        .attr("OPROGRAMMING", "")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .prop(DiffOut("STD", std.name))
                        .bel_attr(bel_other, "IMUX", "")
                        .bel_attr(bel_other, "OPROGRAMMING", "")
                        .bel_attr(bel_other, "OSTANDARD", "")
                        .bel_attr(bel_other, "OUSED", "")
                        .test_manual("OSTD", std.name)
                        .mode_diff("IOB", "IOBM")
                        .pin("O")
                        .attr("OUSED", "0")
                        .attr("DIFFO_OUTUSED", "0")
                        .attr("OSTANDARD", std.name)
                        .bel_mode_diff(bel_other, "IOB", "IOBS")
                        .bel_attr(bel_other, "OUTMUX", "1")
                        .bel_attr(bel_other, "DIFFO_INUSED", "0")
                        .pin_pair("DIFFO_OUT", bel_other, "DIFFO_IN")
                        .commit();
                }
            } else if matches!(
                std.dci,
                DciKind::Output | DciKind::OutputHalf | DciKind::BiSplit | DciKind::BiVcc
            ) {
                bctx.mode("IOB")
                    .pin("O")
                    .attr("IMUX", "")
                    .attr("OPROGRAMMING", "")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .prop(Dci(Some(std.name)))
                    .test_manual("OSTD", std.name)
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", std.name)
                    .commit();
            } else if !std.drive.is_empty() {
                for &drive in std.drive {
                    for slew in ["SLOW", "FAST"] {
                        bctx.mode("IOB")
                            .pin("O")
                            .attr("IMUX", "")
                            .attr("OPROGRAMMING", "")
                            .test_manual("OSTD", format!("{name}.{drive}.{slew}", name = std.name))
                            .attr("OUSED", "0")
                            .attr("OSTANDARD", std.name)
                            .attr("DRIVE", drive)
                            .attr("SLEW", slew)
                            .commit();
                    }
                }
            } else {
                bctx.mode("IOB")
                    .pin("O")
                    .attr("IMUX", "")
                    .attr("OPROGRAMMING", "")
                    .test_manual("OSTD", std.name)
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", std.name)
                    .commit();
            }
        }

        for (std, vref) in [
            ("HSTL_I", 750),
            ("HSTL_III", 900),
            ("HSTL_III_18", 1080),
            ("SSTL2_I", 1250),
        ] {
            bctx.mode("IOB")
                .attr("OUSED", "")
                .pin("I")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .prop(VrefInternal("HCLK_IOI", vref))
                .test_manual("ISTD", std)
                .attr("IMUX", "I_B")
                .attr("ISTANDARD", std)
                .commit();
        }

        bctx.test_manual("OUTPUT_DELAY", "0")
            .pip((bel_ologic, "O_IOB"), (bel_ologic, "OQ"))
            .commit();
        bctx.test_manual("OUTPUT_DELAY", "1")
            .pip((bel_ologic, "O_IOB"), (bel_iodelay, "DATAOUT"))
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        ctx.build()
            .global("ENABLEMISR", "Y")
            .extra_tiles_by_bel(bels::OLOGIC0, "OLOGIC_COMMON")
            .test_manual("OLOGIC_COMMON", "MISR_RESET", "1")
            .global_diff("MISRRESET", "N", "Y")
            .commit();
    }

    for tile in [
        "HCLK_IOI",
        "HCLK_IOI_CENTER",
        "HCLK_IOI_TOPCEN",
        "HCLK_IOI_BOTCEN",
        "HCLK_CMT_IOI",
        "HCLK_IOI_CMT",
    ] {
        if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) {
            let mut bctx = ctx.bel(bels::DCI);
            bctx.build()
                .global_mutex("GLOBAL_DCI", "NOPE")
                .test_manual("TEST_ENABLE", "1")
                .mode("DCI")
                .commit();
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .extra_tiles_by_bel(bels::DCI, "DCI")
        .test_manual("DCI", "QUIET", "1")
        .global_diff("DCIUPDATEMODE", "CONTINUOUS", "QUIET")
        .commit();
    for bank in [3, 4] {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        if bank == 3 && chip.row_bufg() + 30 > chip.rows().next_back().unwrap() {
            continue;
        }
        let mut builder = ctx
            .build()
            .raw(Key::Package, &package.name)
            .extra_tile_attr(
                FixedRelation(edev.node_cfg(die)),
                "MISC",
                "DCI_CLK_ENABLE",
                "1",
            );

        // Find VR and IO rows.
        let (vr_row, io_row) = match bank {
            3 => (chip.row_bufg() + 30 - 3, chip.row_bufg() + 30 - 1),
            4 => (chip.row_bufg() - 30 + 2, chip.row_bufg() - 30),
            _ => unreachable!(),
        };
        let vr_bel = (die, (edev.col_cfg, vr_row), bels::IOB0);
        let vr_node = edev.egrid.get_node_by_bel(vr_bel);
        let io_bel = (die, (edev.col_cfg, io_row), bels::IOB0);
        let io_node = edev.egrid.get_node_by_bel(io_bel);
        let hclk_row = chip.row_hclk(io_row);
        let hclk_node = edev
            .egrid
            .get_node_by_bel((die, (edev.col_cfg, hclk_row), bels::DCI));

        // Ensure nothing is placed in VR.
        for bel in [bels::IOB0, bels::IOB1] {
            let site = backend
                .ngrid
                .get_bel_name((die, (edev.col_cfg, vr_row), bel))
                .unwrap();
            builder = builder.raw(Key::SiteMode(site), None);
        }
        builder = builder.extra_tile_attr_fixed(vr_node, "IOB_COMMON", "PRESENT", "VR");

        // Set up hclk.
        builder = builder.extra_tile_attr_fixed(hclk_node, "DCI", "ENABLE", "1");

        // Set up the IO and fire.
        let site = backend.ngrid.get_bel_name(io_bel).unwrap();
        builder
            .raw(Key::SiteMode(site), "IOB")
            .raw(Key::SitePin(site, "O".into()), true)
            .raw(Key::SiteAttr(site, "IMUX".into()), None)
            .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
            .raw_diff(Key::SiteAttr(site, "OUSED".into()), None, "0")
            .raw_diff(Key::SiteAttr(site, "OSTANDARD".into()), None, "LVDCI_33")
            // Take exclusive mutex on global DCI.
            .raw_diff(Key::GlobalMutex("GLOBAL_DCI".into()), None, "EXCLUSIVE")
            // Avoid interference.
            .raw(Key::GlobalOpt("MATCH_CYCLE".into()), "NOWAIT")
            .extra_tile_attr_fixed(io_node, "IOB0", "OSTD", "LVDCI_33")
            .test_manual("NULL", format!("CENTER_DCI.{bank}"), "1")
            .commit();
    }
    for (bank_from, bank_to) in [(3, 1), (4, 2)] {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        if bank_from == 3 && chip.row_bufg() + 30 > chip.rows().next_back().unwrap() {
            continue;
        }
        let mut builder = ctx.build().raw(Key::Package, &package.name);

        let io_row_from = match bank_from {
            3 => chip.row_bufg() + 30 - 1,
            4 => chip.row_bufg() - 30,
            _ => unreachable!(),
        };
        let io_row_to = match bank_to {
            1 => chip.row_bufg() + 10,
            2 => chip.row_bufg() - 11,
            _ => unreachable!(),
        };
        let io_bel_to = (die, (edev.col_cfg, io_row_to), bels::IOB0);
        let io_node_to = edev.egrid.get_node_by_bel(io_bel_to);
        let hclk_row_to = chip.row_hclk(io_row_to);
        let hclk_node_to =
            edev.egrid
                .get_node_by_bel((die, (edev.col_cfg, hclk_row_to), bels::DCI));

        // Ensure nothing else in the bank.
        let bot = chip.row_reg_bot(chip.row_to_reg(io_row_from));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            for bel in [bels::IOB0, bels::IOB1] {
                if row == io_row_from && bel == bels::IOB0 {
                    continue;
                }
                if let Some(site) = backend.ngrid.get_bel_name((die, (edev.col_cfg, row), bel)) {
                    builder = builder.raw(Key::SiteMode(site), None);
                }
            }
        }
        let site = backend
            .ngrid
            .get_bel_name((die, (edev.col_cfg, io_row_from), bels::IOB0))
            .unwrap();
        builder = builder
            .raw(Key::SiteMode(site), "IOB")
            .raw(Key::SitePin(site, "O".into()), true)
            .raw(Key::SiteAttr(site, "IMUX".into()), None)
            .raw(Key::SiteAttr(site, "IUSED".into()), None)
            .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
            .raw(Key::SiteAttr(site, "OUSED".into()), "0")
            .raw(
                Key::SiteAttr(site, "OSTANDARD".into()),
                if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex6 {
                    "LVDCI_25"
                } else {
                    "LVDCI_33"
                },
            )
            // Take shared mutex on global DCI.
            .raw(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");

        // Ensure nothing else in the bank.
        let bot = chip.row_reg_bot(chip.row_to_reg(io_row_to));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            for bel in [bels::IOB0, bels::IOB1] {
                if row == io_row_to && bel == bels::IOB0 {
                    continue;
                }
                if let Some(site) = backend.ngrid.get_bel_name((die, (edev.col_cfg, row), bel)) {
                    builder = builder.raw(Key::SiteMode(site), None);
                }
            }
        }
        let site = backend.ngrid.get_bel_name(io_bel_to).unwrap();
        builder
            .raw(Key::SiteMode(site), "IOB")
            .raw(Key::SitePin(site, "O".into()), true)
            .raw(Key::SiteAttr(site, "IMUX".into()), None)
            .raw(Key::SiteAttr(site, "IUSED".into()), None)
            .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
            .raw_diff(Key::SiteAttr(site, "OUSED".into()), None, "0")
            .raw_diff(
                Key::SiteAttr(site, "OSTANDARD".into()),
                None,
                if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex6 {
                    "LVDCI_25"
                } else {
                    "LVDCI_33"
                },
            )
            .raw_diff(Key::DciCascade(bank_to), None, bank_from)
            .extra_tile_attr_fixed(io_node_to, "IOB0", "OSTD", "LVDCI_33")
            .extra_tile_attr_fixed(
                hclk_node_to,
                "DCI",
                if bank_to == 1 {
                    "CASCADE_FROM_ABOVE"
                } else {
                    "CASCADE_FROM_BELOW"
                },
                "1",
            )
            .test_manual("NULL", format!("CASCADE_DCI.{bank_from}.{bank_to}"), "1")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let tile = "IO";

    if devdata_only {
        for i in 0..2 {
            let bel = &format!("IODELAY{i}");

            let mut diff_default = ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "DEFAULT");
            let val = extract_bitvec_val_part(
                ctx.tiledb.item(tile, bel, "IDELAY_VALUE_INIT"),
                &bitvec![0; 6],
                &mut diff_default,
            );
            ctx.insert_device_data("IODELAY:DEFAULT_IDELAY_VALUE", val);
            let val = extract_bitvec_val_part(
                ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"),
                &bitvec![0; 6],
                &mut diff_default,
            );
            ctx.insert_device_data("IODELAY:DEFAULT_IDELAY_VALUE", val);
        }
        return;
    }

    {
        let bel = "IOI";
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.ICLK0",
            &[
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "HCLK8",
                "HCLK9", "IOCLK0", "IOCLK1", "IOCLK2", "IOCLK3", "RCLK0", "RCLK1", "RCLK2",
                "RCLK3", "CKINT0", "CKINT1",
            ],
            "NONE",
            OcdMode::Mux,
        );
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.ICLK1",
            &[
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "HCLK8",
                "HCLK9", "IOCLK0", "IOCLK1", "IOCLK2", "IOCLK3", "RCLK0", "RCLK1", "RCLK2",
                "RCLK3", "CKINT0", "CKINT1",
            ],
            "NONE",
            OcdMode::Mux,
        );
        for ibel in ["ILOGIC0", "ILOGIC1"] {
            for attr in ["INV.ICLK0", "INV.ICLK1"] {
                let item = ctx.extract_enum_bool_wide(tile, ibel, attr, "0", "1");
                ctx.tiledb.insert(tile, bel, attr, item);
            }
        }
    }
    for i in 0..2 {
        let bel = &format!("ILOGIC{i}");
        ctx.collect_inv(tile, bel, "CLKDIV");
        ctx.collect_enum(tile, bel, "MUX.CLK", &["ICLK0", "ICLK1"]);
        ctx.collect_enum(tile, bel, "MUX.CLKB", &["ICLK0", "ICLK1"]);

        let diff1 = ctx.state.get_diff(tile, bel, "OCLKINV.DDR", "OCLK_B");
        let diff2 = ctx.state.get_diff(tile, bel, "OCLKINV.DDR", "OCLK");
        ctx.state
            .get_diff(tile, bel, "OCLKINV.SDR", "OCLK")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "OCLKINV.SDR", "OCLK_B");
        diff = diff.combine(&!&diff1);
        diff = diff.combine(&!&diff2);
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "INV.OCLK1", xlat_bit(diff1));
        ctx.tiledb.insert(tile, bel, "INV.OCLK2", xlat_bit(diff2));

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum(tile, bel, "INTERFACE_TYPE", &["MEMORY", "NETWORKING"]);
        ctx.collect_enum(tile, bel, "NUM_CE", &["1", "2"]);
        ctx.collect_bitvec(tile, bel, "INIT_BITSLIPCNT", "");
        ctx.collect_bitvec(tile, bel, "INIT_CE", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK1_PARTIAL", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK2", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK3", "");
        let item = ctx.extract_enum_bool(tile, bel, "SRTYPE", "ASYNC", "SYNC");
        ctx.tiledb.insert(tile, bel, "IFF_SR_SYNC", item);
        for (sattr, attr) in [
            ("INIT_Q1", "IFF1_INIT"),
            ("INIT_Q2", "IFF2_INIT"),
            ("INIT_Q3", "IFF3_INIT"),
            ("INIT_Q4", "IFF4_INIT"),
            ("SRVAL_Q1", "IFF1_SRVAL"),
            ("SRVAL_Q2", "IFF2_SRVAL"),
            ("SRVAL_Q3", "IFF3_SRVAL"),
            ("SRVAL_Q4", "IFF4_SRVAL"),
        ] {
            let item = ctx.extract_enum_bool(tile, bel, sattr, "0", "1");
            ctx.tiledb.insert(tile, bel, attr, item);
        }

        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10"] {
            diffs.push((val, ctx.state.get_diff(tile, bel, "DATA_WIDTH", val)));
        }
        let mut bits = xlat_enum(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.tiledb.insert(
            tile,
            bel,
            "DATA_WIDTH",
            xlat_enum_ocd(diffs, OcdMode::FixedOrder(&bits)),
        );

        ctx.state
            .get_diff(tile, bel, "BITSLIP_ENABLE.ASYNC", "FALSE")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "BITSLIP_ENABLE.SYNC", "FALSE")
            .assert_empty();
        let diff_async = ctx
            .state
            .get_diff(tile, bel, "BITSLIP_ENABLE.ASYNC", "TRUE");
        let diff_sync = ctx.state.get_diff(tile, bel, "BITSLIP_ENABLE.SYNC", "TRUE");
        let diff_sync = diff_sync.combine(&!&diff_async);
        ctx.tiledb
            .insert(tile, bel, "BITSLIP_ENABLE", xlat_bit_wide(diff_async));
        ctx.tiledb
            .insert(tile, bel, "BITSLIP_SYNC", xlat_bit(diff_sync));

        ctx.collect_enum(
            tile,
            bel,
            "DDR_CLK_EDGE",
            &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
        );

        let iff_rev_used = ctx.extract_bit(tile, bel, "REVUSED", "0");
        ctx.tiledb.insert(tile, bel, "IFF_REV_USED", iff_rev_used);
        let iff_sr_used = ctx.extract_bit(tile, bel, "SRUSED", "0");
        ctx.tiledb.insert(tile, bel, "IFF_SR_USED", iff_sr_used);

        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "#LATCH");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "DDR_CLK_EDGE"),
            "SAME_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "#FF");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "DDR_CLK_EDGE"),
            "SAME_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        ctx.tiledb.insert(tile, bel, "IFF_LATCH", xlat_bit(!diff));
        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "DDR");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.tiledb.insert(tile, bel, "IFF_LATCH", xlat_bit(!diff));

        let mut diffs = vec![];
        for val in ["SDR", "DDR"] {
            let mut diff = ctx.state.get_diff(tile, bel, "DATA_RATE", val);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_SR_USED"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_LATCH"), false, true);
            diffs.push((val, diff));
        }
        ctx.tiledb.insert(tile, bel, "DATA_RATE", xlat_enum(diffs));

        let item = ctx.extract_enum(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
        ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = ctx.extract_enum(tile, bel, "D2OFFBYP_SEL", &["GND", "T"]);
        ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = xlat_enum(vec![
            ("T", ctx.state.get_diff(tile, bel, "TFB_USED", "TRUE")),
            ("GND", ctx.state.get_diff(tile, bel, "TFB_USED", "FALSE")),
        ]);
        ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);

        let item = ctx.extract_enum_bool(tile, bel, "IDELMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFFDELMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);

        ctx.state
            .get_diff(tile, bel, "IOBDELAY", "NONE")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IOBDELAY", "IBUF");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IOBDELAY", "IFD");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IOBDELAY", "BOTH");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();

        let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "I_TSBYPASS_ENABLE", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFFMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "IFF_TSBYPASS_ENABLE", item);
        ctx.state
            .get_diff(tile, bel, "OFB_USED", "FALSE")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "OFB_USED", "TRUE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "IFF_TSBYPASS_ENABLE"),
            true,
            false,
        );
        diff.assert_empty();

        ctx.state
            .get_diff(tile, bel, "PRESENT", "ILOGIC")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "ISERDES");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "TSBYPASS_MUX"), "GND", "T");
        diff.assert_empty();

        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK_I",
            TileItem::from_bit(TileBit::new(0, 29, [13, 50][i]), false),
        );
    }

    for i in 0..2 {
        let bel = &format!("OLOGIC{i}");
        let mut present_ologic = ctx.state.get_diff(tile, bel, "PRESENT", "OLOGIC");
        let mut present_oserdes = ctx.state.get_diff(tile, bel, "PRESENT", "OSERDES");

        for attr in ["DDR_CLK_EDGE", "ODDR_CLK_EDGE", "TDDR_CLK_EDGE"] {
            for val in ["SAME_EDGE", "OPPOSITE_EDGE"] {
                ctx.state.get_diff(tile, bel, attr, val).assert_empty();
            }
        }
        ctx.state
            .get_diff(tile, bel, "CLKINV.SAME", "CLK_B")
            .assert_empty();
        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "T1", "T2", "T3", "T4", "CLKDIV",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }
        let diff_clk1 = ctx.state.get_diff(tile, bel, "CLKINV.OPPOSITE", "CLK");
        let diff_clk2 = ctx.state.get_diff(tile, bel, "CLKINV.OPPOSITE", "CLK_B");
        let diff_clk12 = ctx.state.get_diff(tile, bel, "CLKINV.SAME", "CLK");
        assert_eq!(diff_clk12, diff_clk1.combine(&diff_clk2));
        ctx.tiledb
            .insert(tile, bel, "INV.CLK1", xlat_bit(!diff_clk1));
        ctx.tiledb
            .insert(tile, bel, "INV.CLK2", xlat_bit(!diff_clk2));

        let osrused = ctx.extract_bit(tile, bel, "OSRUSED", "0");
        let tsrused = ctx.extract_bit(tile, bel, "TSRUSED", "0");
        let orevused = ctx.extract_bit(tile, bel, "OREVUSED", "0");
        let trevused = ctx.extract_bit(tile, bel, "TREVUSED", "0");
        ctx.state.get_diff(tile, bel, "OCEUSED", "0").assert_empty();
        ctx.state.get_diff(tile, bel, "TCEUSED", "0").assert_empty();

        let diff_d1 = ctx.state.get_diff(tile, bel, "OMUX", "D1");
        let diff_serdes_sdr = ctx
            .state
            .get_diff(tile, bel, "DATA_RATE_OQ", "SDR")
            .combine(&diff_d1);
        let diff_serdes_ddr = ctx
            .state
            .get_diff(tile, bel, "DATA_RATE_OQ", "DDR")
            .combine(&diff_d1);
        let (diff_serdes_sdr, diff_serdes_ddr, mut diff_off_serdes) =
            Diff::split(diff_serdes_sdr, diff_serdes_ddr);
        diff_off_serdes.apply_bit_diff(&osrused, true, false);
        diff_off_serdes.apply_bit_diff(&orevused, true, false);
        let diff_latch = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#LATCH");
        let diff_ff = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#FF");
        let diff_ddr = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "DDR");
        ctx.state
            .get_diff(tile, bel, "OMUX", "OUTFF")
            .assert_empty();
        present_oserdes = present_oserdes.combine(&!&diff_d1);
        ctx.tiledb.insert(
            tile,
            bel,
            "OMUX",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                ("D1", diff_d1),
                ("SERDES_SDR", diff_serdes_sdr),
                ("SERDES_DDR", diff_serdes_ddr),
                ("FF", diff_ff),
                ("DDR", diff_ddr),
                ("LATCH", diff_latch),
            ]),
        );
        ctx.tiledb
            .insert(tile, bel, "OFF_SERDES", xlat_bit_wide(diff_off_serdes));

        let diff_t1 = ctx.state.get_diff(tile, bel, "TMUX", "T1");
        let diff_serdes_buf = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "BUF");
        let mut diff_serdes_sdr = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "SDR");
        let mut diff_serdes_ddr = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "DDR");
        diff_serdes_sdr.apply_bit_diff(&tsrused, true, false);
        diff_serdes_sdr.apply_bit_diff(&trevused, true, false);
        diff_serdes_ddr.apply_bit_diff(&tsrused, true, false);
        diff_serdes_ddr.apply_bit_diff(&trevused, true, false);
        let diff_latch = ctx.state.get_diff(tile, bel, "TFFTYPE", "#LATCH");
        let diff_ff = ctx.state.get_diff(tile, bel, "TFFTYPE", "#FF");
        let diff_ddr = ctx.state.get_diff(tile, bel, "TFFTYPE", "DDR");
        ctx.state.get_diff(tile, bel, "TMUX", "TFF").assert_empty();
        present_oserdes = present_oserdes.combine(&!&diff_t1);
        present_ologic = present_ologic.combine(&!&diff_t1);
        ctx.tiledb.insert(
            tile,
            bel,
            "TMUX",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                ("T1", diff_t1),
                ("T1", diff_serdes_buf),
                ("SERDES_DDR", diff_serdes_ddr),
                ("FF", diff_serdes_sdr),
                ("FF", diff_ff),
                ("DDR", diff_ddr),
                ("LATCH", diff_latch),
            ]),
        );

        ctx.collect_bitvec(tile, bel, "INIT_LOADCNT", "");
        present_oserdes.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "INIT_LOADCNT"),
            &bitvec![0; 4],
            &bitvec![1; 4],
        );

        present_ologic.assert_empty();
        present_oserdes.assert_empty();

        ctx.tiledb.insert(tile, bel, "OFF_SR_USED", osrused);
        ctx.tiledb.insert(tile, bel, "TFF_SR_USED", tsrused);
        ctx.tiledb.insert(tile, bel, "OFF_REV_USED", orevused);
        ctx.tiledb.insert(tile, bel, "TFF_REV_USED", trevused);

        let item_oq = ctx.extract_enum_bool_wide(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_enum_bool_wide(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.state
            .get_diff(tile, bel, "SRTYPE", "ASYNC")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bitvec_diff(&item_oq, &bitvec![1; 4], &bitvec![0; 4]);
        diff.apply_bitvec_diff(&item_tq, &bitvec![1; 2], &bitvec![0; 2]);
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "OFF_SR_SYNC", item_oq);
        ctx.tiledb.insert(tile, bel, "TFF_SR_SYNC", item_tq);

        let diff_ologic = ctx.state.get_diff(tile, bel, "INIT_OQ.OLOGIC", "0");
        let diff_oserdes = ctx
            .state
            .get_diff(tile, bel, "INIT_OQ.OSERDES", "0")
            .combine(&!&diff_ologic);
        ctx.tiledb
            .insert(tile, bel, "OFF_INIT", xlat_bit_wide(!diff_ologic));
        ctx.tiledb
            .insert(tile, bel, "OFF_INIT_SERDES", xlat_bit_wide(!diff_oserdes));
        ctx.state
            .get_diff(tile, bel, "INIT_OQ.OLOGIC", "1")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "INIT_OQ.OSERDES", "1")
            .assert_empty();
        let item = ctx.extract_enum_bool_wide(tile, bel, "INIT_TQ.OLOGIC", "0", "1");
        ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "INIT_TQ.OSERDES", "0", "1");
        ctx.tiledb.insert(tile, bel, "TFF_INIT", item);

        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_OQ.OLOGIC", "0", "1");
        ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_OQ.OSERDES", "0", "1");
        ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);

        for attr in ["SRVAL_TQ.FF", "SRVAL_TQ.DDR", "SRVAL_TQ.OSERDES"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
        let diff1 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.FF", "0");
        let diff2 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.DDR", "0");
        let diff3 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.OSERDES", "0");
        assert_eq!(diff2, diff3);
        let diff2 = diff2.combine(&!&diff1);
        ctx.tiledb.insert(tile, bel, "TFF1_SRVAL", xlat_bit(!diff1));
        ctx.tiledb
            .insert(tile, bel, "TFF23_SRVAL", xlat_bit_wide(!diff2));

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum(tile, bel, "TRISTATE_WIDTH", &["1", "4"]);
        ctx.collect_enum(
            tile,
            bel,
            "DATA_WIDTH",
            &["2", "3", "4", "5", "6", "7", "8", "10"],
        );

        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE_FDBK", "FALSE", "TRUE");
        ctx.collect_enum_default(tile, bel, "MISR_CLK_SELECT", &["CLK1", "CLK2"], "NONE");

        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.CLK",
            &[
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "HCLK8",
                "HCLK9", "IOCLK0", "IOCLK1", "IOCLK2", "IOCLK3", "RCLK0", "RCLK1", "RCLK2",
                "RCLK3", "CKINT",
            ],
            "NONE",
            OcdMode::Mux,
        );
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.CLKDIV",
            &[
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "HCLK8",
                "HCLK9", "RCLK0", "RCLK1", "RCLK2", "RCLK3", "CKINT",
            ],
            "NONE",
            OcdMode::Mux,
        );
    }
    let mut diff = ctx.state.get_diff(tile, "OLOGIC_COMMON", "MISR_RESET", "1");
    let diff1 = diff.split_bits_by(|bit| bit.bit >= 32);
    ctx.tiledb
        .insert(tile, "OLOGIC0", "MISR_RESET", xlat_bit(diff));
    ctx.tiledb
        .insert(tile, "OLOGIC1", "MISR_RESET", xlat_bit(diff1));

    for i in 0..2 {
        let bel = &format!("IODELAY{i}");
        let item = ctx.extract_inv(tile, bel, "C");
        ctx.tiledb
            .insert(tile, format!("ILOGIC{i}"), "INV.CLKDIV", item);
        ctx.collect_inv(tile, bel, "DATAIN");
        ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "DELAYCHAIN_OSC", "FALSE", "TRUE");
        ctx.collect_enum_default(tile, bel, "DELAY_SRC", &["I", "O", "IO", "DATAIN"], "NONE");
        ctx.collect_bitvec(tile, bel, "ODELAY_VALUE", "");

        let mut diffs_a = vec![];
        let mut diffs_b = vec![];
        for diff in ctx.state.get_diffs(tile, bel, "IDELAY_VALUE", "") {
            let mut diff_a = Diff::default();
            let mut diff_b = Diff::default();
            for (k, v) in diff.bits {
                if v {
                    diff_a.bits.insert(k, v);
                } else {
                    diff_b.bits.insert(k, v);
                }
            }
            diffs_a.push(diff_a);
            diffs_b.push(diff_b);
        }
        ctx.tiledb
            .insert(tile, bel, "IDELAY_VALUE_INIT", xlat_bitvec(diffs_a));
        ctx.tiledb
            .insert(tile, bel, "IDELAY_VALUE_CUR", xlat_bitvec(diffs_b));

        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x3f);
        present.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "NONE", "DATAIN");
        ctx.tiledb
            .insert(tile, bel, "ENABLE", xlat_bit_wide(present));

        let diff = ctx.state.get_diff(tile, bel, "LEGIDELAY", "DISABLE");
        ctx.tiledb.insert(tile, bel, "LEGIDELAY", xlat_bit(!diff));

        ctx.state
            .get_diff(tile, bel, "IDELAY_TYPE", "FIXED")
            .assert_empty();
        let diff_variable = ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "VARIABLE");
        let mut diff_default = ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "DEFAULT");
        let val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "IDELAY_VALUE_INIT"),
            &bitvec![0; 6],
            &mut diff_default,
        );
        ctx.insert_device_data("IODELAY:DEFAULT_IDELAY_VALUE", val);
        let val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"),
            &bitvec![0; 6],
            &mut diff_default,
        );
        ctx.insert_device_data("IODELAY:DEFAULT_IDELAY_VALUE", val);
        ctx.tiledb.insert(
            tile,
            bel,
            "IDELAY_TYPE",
            xlat_enum(vec![
                ("VARIABLE", diff_variable),
                ("FIXED", Diff::default()),
                ("DEFAULT", diff_default),
            ]),
        );
    }

    let mut present_vr = ctx.state.get_diff(tile, "IOB_COMMON", "PRESENT", "VR");
    for i in 0..2 {
        let bel = &format!("IOB{i}");
        ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
        ctx.collect_enum_bool(tile, bel, "OUTPUT_DELAY", "0", "1");
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "IOB");
        let diff = ctx
            .state
            .get_diff(tile, bel, "PRESENT", "IPAD")
            .combine(&!&present);
        ctx.tiledb.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
        let diff = ctx
            .state
            .get_diff(tile, bel, "PRESENT", "IOB.CONTINUOUS")
            .combine(&!&present);
        ctx.tiledb
            .insert(tile, bel, "DCIUPDATEMODE_ASREQUIRED", xlat_bit(!diff));
        present.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");
        let diff = ctx
            .state
            .peek_diff(tile, bel, "OSTD", "LVCMOS25.12.SLOW")
            .combine(&present);
        ctx.tiledb
            .insert(tile, bel, "OUTPUT_ENABLE", xlat_bit_wide(diff));

        let oprog = ctx.extract_bitvec(tile, bel, "OPROGRAMMING", "");
        let lvds = TileItem::from_bitvec(oprog.bits[0..9].to_vec(), false);
        let dci_t = TileItem::from_bit(oprog.bits[9], false);
        let dci_mode = TileItem {
            bits: oprog.bits[10..13].to_vec(),
            kind: TileItemKind::Enum {
                values: [
                    ("NONE".into(), bitvec![0, 0, 0]),
                    ("OUTPUT".into(), bitvec![1, 0, 0]),
                    ("OUTPUT_HALF".into(), bitvec![0, 1, 0]),
                    ("TERM_VCC".into(), bitvec![1, 1, 0]),
                    ("TERM_SPLIT".into(), bitvec![0, 0, 1]),
                ]
                .into_iter()
                .collect(),
            },
        };
        let output_misc = TileItem::from_bitvec(oprog.bits[13..19].to_vec(), false);
        let dci_misc = TileItem::from_bitvec(oprog.bits[19..21].to_vec(), false);
        let pdrive_bits = oprog.bits[21..26].to_vec();
        let ndrive_bits = oprog.bits[26..31].to_vec();
        let pdrive_invert: BitVec = pdrive_bits
            .iter()
            .map(|&bit| match present.bits.remove(&bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            })
            .collect();
        let ndrive_invert: BitVec = ndrive_bits
            .iter()
            .map(|&bit| match present.bits.remove(&bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            })
            .collect();
        let (pslew_bits, nslew_bits) = if i == 0 {
            (
                vec![
                    TileBit::new(0, 37, 17),
                    TileBit::new(0, 36, 23),
                    TileBit::new(0, 37, 23),
                    TileBit::new(0, 37, 30),
                    TileBit::new(0, 37, 29),
                    TileBit::new(0, 37, 27),
                ],
                vec![
                    TileBit::new(0, 36, 31),
                    TileBit::new(0, 36, 27),
                    TileBit::new(0, 37, 31),
                    TileBit::new(0, 37, 28),
                    TileBit::new(0, 36, 26),
                    TileBit::new(0, 37, 20),
                ],
            )
        } else {
            (
                vec![
                    TileBit::new(0, 37, 46),
                    TileBit::new(0, 36, 40),
                    TileBit::new(0, 37, 40),
                    TileBit::new(0, 37, 33),
                    TileBit::new(0, 37, 34),
                    TileBit::new(0, 37, 36),
                ],
                vec![
                    TileBit::new(0, 36, 32),
                    TileBit::new(0, 36, 36),
                    TileBit::new(0, 37, 32),
                    TileBit::new(0, 37, 35),
                    TileBit::new(0, 36, 37),
                    TileBit::new(0, 37, 43),
                ],
            )
        };
        let pslew = TileItem::from_bitvec(pslew_bits, false);
        let nslew = TileItem::from_bitvec(nslew_bits, false);

        let diff_cmos = ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS18");
        let diff_vref = ctx.state.peek_diff(tile, bel, "ISTD", "HSTL_I");
        let diff_diff = ctx.state.peek_diff(tile, bel, "ISTD", "LVDS_25");
        let (_, _, diff_diff) = Diff::split(diff_cmos.clone(), diff_diff.clone());
        let item = xlat_enum(vec![
            ("OFF", Diff::default()),
            ("CMOS", diff_cmos.clone()),
            ("VREF", diff_vref.clone()),
            ("DIFF", diff_diff),
        ]);
        ctx.tiledb.insert(tile, bel, "IBUF_MODE", item);

        for &std in IOSTDS {
            if std.diff == DiffKind::True {
                continue;
            }
            let (drives, slews) = if !std.drive.is_empty() {
                (std.drive, &["SLOW", "FAST"][..])
            } else {
                (&[""][..], &[""][..])
            };
            for &drive in drives {
                for &slew in slews {
                    let val = if drive.is_empty() {
                        std.name.to_string()
                    } else {
                        format!("{name}.{drive}.{slew}", name = std.name)
                    };
                    let mut diff = ctx.state.get_diff(tile, bel, "OSTD", val);
                    diff.apply_bitvec_diff(
                        ctx.tiledb.item(tile, bel, "OUTPUT_ENABLE"),
                        &bitvec![1; 2],
                        &bitvec![0; 2],
                    );
                    let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                    if !matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                        for (attr, bits, invert) in [
                            ("PDRIVE", &pdrive_bits, &pdrive_invert),
                            ("NDRIVE", &ndrive_bits, &ndrive_invert),
                        ] {
                            let value: BitVec = bits
                                .iter()
                                .zip(invert.iter())
                                .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                    Some(val) => {
                                        assert_eq!(val, !*inv);
                                        true
                                    }
                                    None => false,
                                })
                                .collect();
                            let name = if drive.is_empty() {
                                stdname.to_string()
                            } else {
                                format!("{stdname}.{drive}")
                            };
                            ctx.tiledb
                                .insert_misc_data(format!("IOSTD:{attr}:{name}"), value);
                        }
                    }
                    for (attr, item) in [("PSLEW", &pslew), ("NSLEW", &nslew)] {
                        let value: BitVec = item
                            .bits
                            .iter()
                            .map(|&bit| match diff.bits.remove(&bit) {
                                Some(true) => true,
                                None => false,
                                _ => unreachable!(),
                            })
                            .collect();
                        let name = if drive.is_empty() {
                            stdname.to_string()
                        } else {
                            format!("{stdname}.{drive}.{slew}")
                        };
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{attr}:{name}"), value);
                    }
                    let value: BitVec = output_misc
                        .bits
                        .iter()
                        .map(|&bit| match diff.bits.remove(&bit) {
                            Some(true) => true,
                            None => false,
                            _ => unreachable!(),
                        })
                        .collect();
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:OUTPUT_MISC:{stdname}"), value);
                    match std.dci {
                        DciKind::None | DciKind::InputVcc | DciKind::InputSplit => {}
                        DciKind::Output => {
                            diff.apply_enum_diff(&dci_mode, "OUTPUT", "NONE");
                        }
                        DciKind::OutputHalf => {
                            diff.apply_enum_diff(&dci_mode, "OUTPUT_HALF", "NONE");
                        }
                        DciKind::BiVcc => {
                            diff.apply_enum_diff(&dci_mode, "TERM_VCC", "NONE");
                            diff.apply_bitvec_diff(&dci_misc, &bitvec![1, 1], &bitvec![0, 0]);
                        }
                        DciKind::BiSplit => {
                            diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                        }
                        DciKind::BiSplitT => {
                            diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                            diff.apply_bit_diff(&dci_t, true, false);
                        }
                    }
                    diff.assert_empty();
                }
            }
        }

        for (attr, bits, invert) in [
            ("PDRIVE", &pdrive_bits, &pdrive_invert),
            ("NDRIVE", &ndrive_bits, &ndrive_invert),
            ("PSLEW", &pslew.bits, &bitvec![0; 6]),
            ("NSLEW", &nslew.bits, &bitvec![0; 6]),
        ] {
            let value: BitVec = bits
                .iter()
                .zip(invert.iter())
                .map(|(&bit, inv)| match present_vr.bits.remove(&bit) {
                    Some(true) => !*inv,
                    None => *inv,
                    _ => unreachable!(),
                })
                .collect();
            ctx.tiledb
                .insert_misc_data(format!("IOSTD:{attr}:VR"), value);
        }
        present_vr.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");
        present_vr.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");

        if i == 0 {
            let mut present_vref = ctx.state.get_diff(tile, bel, "PRESENT", "VREF");
            present_vref.apply_bit_diff(ctx.tiledb.item(tile, bel, "VREF_SYSMON"), true, false);
            present_vref.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

            for (attr, bits, invert) in [
                ("PDRIVE", &pdrive_bits, &pdrive_invert),
                ("NDRIVE", &ndrive_bits, &ndrive_invert),
                ("PSLEW", &pslew.bits, &bitvec![0; 6]),
                ("NSLEW", &nslew.bits, &bitvec![0; 6]),
            ] {
                let value: BitVec = bits
                    .iter()
                    .zip(invert.iter())
                    .map(|(&bit, inv)| match present_vref.bits.remove(&bit) {
                        Some(true) => !*inv,
                        None => *inv,
                        _ => unreachable!(),
                    })
                    .collect();
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:{attr}:OFF"), value);
            }
            present_vref.assert_empty();
        }

        ctx.tiledb
            .insert_misc_data("IOSTD:OUTPUT_MISC:OFF", bitvec![0; 6]);
        ctx.tiledb
            .insert_misc_data("IOSTD:LVDS_T:OFF", bitvec![0; 9]);
        ctx.tiledb
            .insert_misc_data("IOSTD:LVDS_C:OFF", bitvec![0; 9]);
        ctx.tiledb
            .insert_misc_data("IOSTD:PDRIVE:OFF", bitvec![0; 5]);
        ctx.tiledb
            .insert_misc_data("IOSTD:NDRIVE:OFF", bitvec![0; 5]);
        ctx.tiledb
            .insert_misc_data("IOSTD:PSLEW:OFF", bitvec![0; 6]);
        ctx.tiledb
            .insert_misc_data("IOSTD:NSLEW:OFF", bitvec![0; 6]);
        ctx.tiledb.insert(tile, bel, "LVDS", lvds);
        ctx.tiledb.insert(tile, bel, "DCI_T", dci_t);
        ctx.tiledb.insert(tile, bel, "DCI_MODE", dci_mode);
        ctx.tiledb.insert(tile, bel, "OUTPUT_MISC", output_misc);
        ctx.tiledb.insert(tile, bel, "DCI_MISC", dci_misc);
        ctx.tiledb.insert(
            tile,
            bel,
            "PDRIVE",
            TileItem {
                bits: pdrive_bits,
                kind: TileItemKind::BitVec {
                    invert: pdrive_invert,
                },
            },
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "NDRIVE",
            TileItem {
                bits: ndrive_bits,
                kind: TileItemKind::BitVec {
                    invert: ndrive_invert,
                },
            },
        );
        ctx.tiledb.insert(tile, bel, "PSLEW", pslew);
        ctx.tiledb.insert(tile, bel, "NSLEW", nslew);

        let diff = ctx.state.get_diff(tile, bel, "IMUX", "I");
        ctx.tiledb.insert(tile, bel, "INV.I", xlat_bit(!diff));

        present.assert_empty();
    }
    let diff1 = present_vr.split_bits_by(|bit| bit.bit >= 32);
    ctx.tiledb.insert(tile, "IOB0", "VR", xlat_bit(present_vr));
    ctx.tiledb.insert(tile, "IOB1", "VR", xlat_bit(diff1));
    for i in 0..2 {
        let bel = &format!("IOB{i}");
        for &std in IOSTDS {
            let mut diff = ctx.state.get_diff(tile, bel, "ISTD", std.name);
            if std.diff != DiffKind::None {
                for bel in ["IOB0", "IOB1"] {
                    match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                        DciKind::InputVcc | DciKind::BiVcc => {
                            diff.apply_enum_diff(
                                ctx.tiledb.item(tile, bel, "DCI_MODE"),
                                "TERM_VCC",
                                "NONE",
                            );
                            diff.apply_bitvec_diff(
                                ctx.tiledb.item(tile, bel, "DCI_MISC"),
                                &bitvec![1, 1],
                                &bitvec![0, 0],
                            );
                        }
                        DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                            diff.apply_enum_diff(
                                ctx.tiledb.item(tile, bel, "DCI_MODE"),
                                "TERM_SPLIT",
                                "NONE",
                            );
                        }
                    }
                    diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IBUF_MODE"), "DIFF", "OFF");
                }
                diff.assert_empty();
            } else {
                match std.dci {
                    DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                    DciKind::InputVcc | DciKind::BiVcc => {
                        diff.apply_enum_diff(
                            ctx.tiledb.item(tile, bel, "DCI_MODE"),
                            "TERM_VCC",
                            "NONE",
                        );
                        diff.apply_bitvec_diff(
                            ctx.tiledb.item(tile, bel, "DCI_MISC"),
                            &bitvec![1, 1],
                            &bitvec![0, 0],
                        );
                    }
                    DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                        diff.apply_enum_diff(
                            ctx.tiledb.item(tile, bel, "DCI_MODE"),
                            "TERM_SPLIT",
                            "NONE",
                        );
                    }
                }
                let mode = if std.vref.is_some() { "VREF" } else { "CMOS" };
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IBUF_MODE"), mode, "OFF");
                diff.assert_empty();
            }
            if std.diff == DiffKind::True {
                let mut diff = ctx.state.get_diff(tile, bel, "DIFF_TERM", std.name);
                let val_c = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB0", "LVDS"),
                    &bitvec![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB1", "LVDS"),
                    &bitvec![0; 9],
                    &mut diff,
                );
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                diff.assert_empty();
                if i == 1 {
                    let mut diff = ctx.state.get_diff(tile, bel, "OSTD", std.name);
                    let val_c = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, "IOB0", "LVDS"),
                        &bitvec![0; 9],
                        &mut diff,
                    );
                    let val_t = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, "IOB1", "LVDS"),
                        &bitvec![0; 9],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:LVDS_T:OUTPUT_{}", std.name), val_t);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:LVDS_C:OUTPUT_{}", std.name), val_c);
                    diff.apply_bitvec_diff(
                        ctx.tiledb.item(tile, "IOB1", "OUTPUT_ENABLE"),
                        &bitvec![1; 2],
                        &bitvec![0; 2],
                    );
                    diff.assert_empty();
                }
            }
        }
    }

    let lvdsbias = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 35, 15),
            TileBit::new(0, 34, 15),
            TileBit::new(0, 34, 14),
            TileBit::new(0, 35, 14),
            TileBit::new(0, 35, 13),
            TileBit::new(0, 34, 13),
            TileBit::new(0, 34, 12),
            TileBit::new(0, 35, 12),
            TileBit::new(0, 32, 13),
            TileBit::new(0, 33, 13),
            TileBit::new(0, 33, 12),
            TileBit::new(0, 32, 12),
        ],
        false,
    );
    let lvdiv2 = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 52, 12),
            TileBit::new(0, 53, 12),
            TileBit::new(0, 53, 15),
        ],
        false,
    );
    let pref = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 51, 12),
            TileBit::new(0, 50, 12),
            TileBit::new(0, 53, 14),
            TileBit::new(0, 52, 15),
        ],
        false,
    );
    let nref = TileItem::from_bitvec(
        vec![TileBit::new(0, 52, 14), TileBit::new(0, 52, 13)],
        false,
    );
    let pmask_term_vcc = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 50, 15),
            TileBit::new(0, 50, 14),
            TileBit::new(0, 51, 14),
            TileBit::new(0, 51, 13),
            TileBit::new(0, 50, 13),
        ],
        false,
    );
    let pmask_term_split = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 46, 13),
            TileBit::new(0, 46, 12),
            TileBit::new(0, 47, 12),
            TileBit::new(0, 48, 15),
            TileBit::new(0, 49, 15),
        ],
        false,
    );
    let nmask_term_split = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 48, 13),
            TileBit::new(0, 49, 13),
            TileBit::new(0, 49, 12),
            TileBit::new(0, 48, 12),
            TileBit::new(0, 51, 15),
        ],
        false,
    );
    let vref = ctx.extract_enum_default(
        "HCLK_IOI",
        "INTERNAL_VREF",
        "VREF",
        &["750", "900", "1080", "1250"],
        "OFF",
    );
    let dci_en = ctx.extract_bit("HCLK_IOI_CMT", "DCI", "ENABLE", "1");
    let has_bank3 = ctx.has_tile("HCLK_CMT_IOI");
    if has_bank3 {
        let dci_en_too = ctx.extract_bit("HCLK_CMT_IOI", "DCI", "ENABLE", "1");
        assert_eq!(dci_en, dci_en_too);
    }
    let dci_casc_above = if has_bank3 {
        Some(ctx.extract_bit("HCLK_IOI_TOPCEN", "DCI", "CASCADE_FROM_ABOVE", "1"))
    } else {
        None
    };
    let dci_casc_below = ctx.extract_bit("HCLK_IOI_BOTCEN", "DCI", "CASCADE_FROM_BELOW", "1");
    for tile in [
        "HCLK_IOI",
        "HCLK_IOI_CENTER",
        "HCLK_IOI_BOTCEN",
        "HCLK_IOI_TOPCEN",
        "HCLK_IOI_CMT",
        "HCLK_CMT_IOI",
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "LVDS";
        ctx.tiledb.insert(tile, bel, "LVDSBIAS", lvdsbias.clone());
        let bel = "INTERNAL_VREF";
        ctx.tiledb.insert(tile, bel, "VREF", vref.clone());
        let bel = "DCI";
        ctx.tiledb.insert(tile, bel, "ENABLE", dci_en.clone());
        if let Some(ref dci_casc_above) = dci_casc_above {
            ctx.tiledb
                .insert(tile, bel, "CASCADE_FROM_ABOVE", dci_casc_above.clone());
        }
        ctx.tiledb
            .insert(tile, bel, "CASCADE_FROM_BELOW", dci_casc_below.clone());
        ctx.tiledb.insert(tile, bel, "LVDIV2", lvdiv2.clone());
        ctx.tiledb.insert(tile, bel, "PREF", pref.clone());
        ctx.tiledb.insert(tile, bel, "NREF", nref.clone());
        ctx.tiledb
            .insert(tile, bel, "PMASK_TERM_VCC", pmask_term_vcc.clone());
        ctx.tiledb
            .insert(tile, bel, "PMASK_TERM_SPLIT", pmask_term_split.clone());
        ctx.tiledb
            .insert(tile, bel, "NMASK_TERM_SPLIT", nmask_term_split.clone());
        ctx.collect_bit_wide(tile, bel, "TEST_ENABLE", "1");
        ctx.collect_bit(tile, bel, "QUIET", "1");
    }
    let tile = "HCLK_IOI";
    for std in IOSTDS {
        if std.diff == DiffKind::True {
            let bel = "LVDS";
            let diff = ctx.state.get_diff(tile, bel, "STD", std.name);
            let val = extract_bitvec_val(&lvdsbias, &bitvec![0; 12], diff);
            ctx.tiledb
                .insert_misc_data(format!("IOSTD:LVDSBIAS:{}", std.name), val);
        }
        if std.dci != DciKind::None {
            let bel = "DCI";
            let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
            let mut diff = ctx.state.get_diff(tile, bel, "STD", std.name);
            match std.dci {
                DciKind::OutputHalf => {
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, "LVDIV2"),
                        &bitvec![0; 3],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:LVDIV2:{stdname}"), val);
                }
                DciKind::InputVcc | DciKind::BiVcc => {
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, "PMASK_TERM_VCC"),
                        &bitvec![0; 5],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_VCC:{stdname}"), val);
                }
                DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, "PMASK_TERM_SPLIT"),
                        &bitvec![0; 5],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_SPLIT:{stdname}"), val);
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, "NMASK_TERM_SPLIT"),
                        &bitvec![0; 5],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:NMASK_TERM_SPLIT:{stdname}"), val);
                }
                _ => {}
            }
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff));
        }
    }
    ctx.tiledb
        .insert_misc_data("IOSTD:LVDSBIAS:OFF", bitvec![0; 12]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:LVDIV2:OFF", bitvec![0; 3]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PMASK_TERM_VCC:OFF", bitvec![0; 5]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PMASK_TERM_SPLIT:OFF", bitvec![0; 5]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:NMASK_TERM_SPLIT:OFF", bitvec![0; 5]);
    let tile = "CFG";
    let bel = "MISC";
    ctx.collect_bit_wide(tile, bel, "DCI_CLK_ENABLE", "1");
}
