#![recursion_limit = "1024"]

use prjcombine_interconnect::db::BelInfo;
use prjcombine_interconnect::dir::DirH;
use prjcombine_interconnect::grid::{CellCoord, DieId, RowId, TileIobId};
use prjcombine_re_xilinx_naming_ultrascale::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier, verify};
use prjcombine_ultrascale::bels;
use prjcombine_ultrascale::bond::SharedCfgPad;
use prjcombine_ultrascale::chip::{
    Chip, ChipKind, CleMKind, ColumnKind, ConfigKind, DisabledPart, IoRowKind,
};
use prjcombine_ultrascale::expanded::{ClkSrc, HdioCoord, HpioCoord, IoCoord};
use unnamed_entity::EntityId;

mod xp5io;

fn is_cut_d(endev: &ExpandedNamedDevice, die: DieId, row: RowId) -> bool {
    let reg = endev.edev.chips[die].row_to_reg(row);
    if reg.to_idx() == 0 {
        false
    } else {
        endev
            .edev
            .disabled
            .contains(&DisabledPart::Region(die, reg - 1))
    }
}

fn is_cut_u(endev: &ExpandedNamedDevice, die: DieId, row: RowId) -> bool {
    let reg = endev.edev.chips[die].row_to_reg(row);
    endev
        .edev
        .disabled
        .contains(&DisabledPart::Region(die, reg + 1))
}

fn get_hpiob_bel<'a>(
    endev: &ExpandedNamedDevice,
    vrf: &Verifier<'a>,
    crd: IoCoord,
) -> BelContext<'a> {
    let IoCoord::Hpio(crd) = crd else {
        unreachable!();
    };
    let chip = endev.edev.chips[crd.die];
    let row = chip.row_reg_bot(crd.reg) + if crd.iob.to_idx() < 26 { 0 } else { 30 };
    vrf.find_bel(CellCoord::new(crd.die, crd.col, row).bel(bels::HPIOB[crd.iob.to_idx() % 26]))
        .or_else(|| {
            vrf.find_bel(
                CellCoord::new(crd.die, crd.col, row).bel(bels::HRIOB[crd.iob.to_idx() % 26]),
            )
        })
        .unwrap()
}

fn verify_slice(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.tcls == "CLEM" {
        "SLICEM"
    } else {
        "SLICEL"
    };
    if bel.name.is_some() {
        vrf.verify_bel(
            bel,
            kind,
            &[("CIN", SitePinDir::In), ("COUT", SitePinDir::Out)],
            &[],
        );
    }
    vrf.claim_pip(bel.crd(), bel.wire("CIN"), bel.wire_far("CIN"));
    vrf.claim_node(&[bel.fwire("CIN")]);
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.slot) {
        vrf.verify_node(&[bel.fwire_far("CIN"), obel.fwire("COUT")]);
    } else if !is_cut_d(endev, bel.die, bel.row)
        || (kind == "SLICEM"
            && endev.edev.chips[bel.die].columns[bel.col].kind
                == ColumnKind::CleM(CleMKind::Laguna)
            && bel.row.to_idx() == Chip::ROWS_PER_REG)
    {
        vrf.claim_node(&[bel.fwire_far("CIN")]);
    }
    vrf.claim_node(&[bel.fwire("COUT")]);
}

fn verify_dsp(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    pairs.push(("MULTSIGNIN".to_string(), "MULTSIGNOUT".to_string()));
    pairs.push(("CARRYCASCIN".to_string(), "CARRYCASCOUT".to_string()));
    for i in 0..30 {
        pairs.push((format!("ACIN_B{i}"), format!("ACOUT_B{i}")));
    }
    for i in 0..18 {
        pairs.push((format!("BCIN_B{i}"), format!("BCOUT_B{i}")));
    }
    for i in 0..48 {
        pairs.push((format!("PCIN{i}"), format!("PCOUT{i}")));
    }
    let mut pins = vec![];
    for (ipin, opin) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        if bel.slot == bels::DSP0 {
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bels::DSP1) {
                vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            } else if !is_cut_d(endev, bel.die, bel.row) {
                vrf.claim_node(&[bel.fwire_far(ipin)]);
            }
        } else {
            let obel = vrf.find_bel_sibling(bel, bels::DSP0);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    if bel.name.is_some() {
        vrf.verify_bel(bel, "DSP48E2", &pins, &[]);
    }
}

fn verify_bram_f(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum Mode {
        Up,
        DownHalfReg,
        UpBuf,
        DownBuf,
    }
    let mut pairs = vec![
        (
            "ENABLE_BIST".to_string(),
            "START_RSR_NEXT".to_string(),
            Mode::DownHalfReg,
        ),
        (
            "CASINSBITERR".to_string(),
            "CASOUTSBITERR".to_string(),
            Mode::Up,
        ),
        (
            "CASINDBITERR".to_string(),
            "CASOUTDBITERR".to_string(),
            Mode::Up,
        ),
        (
            "CASPRVEMPTY".to_string(),
            "CASNXTEMPTY".to_string(),
            Mode::UpBuf,
        ),
        (
            "CASNXTRDEN".to_string(),
            "CASPRVRDEN".to_string(),
            Mode::DownBuf,
        ),
    ];
    for ab in ['A', 'B'] {
        for ul in ['U', 'L'] {
            for i in 0..16 {
                pairs.push((
                    format!("CASDI{ab}{ul}{i}"),
                    format!("CASDO{ab}{ul}{i}"),
                    Mode::UpBuf,
                ));
            }
            for i in 0..2 {
                pairs.push((
                    format!("CASDIP{ab}{ul}{i}"),
                    format!("CASDOP{ab}{ul}{i}"),
                    Mode::UpBuf,
                ));
            }
        }
    }
    let mut pins = vec![("CASMBIST12OUT", SitePinDir::Out)];
    vrf.claim_node(&[bel.fwire("CASMBIST12OUT")]);
    for (ipin, opin, mode) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
        match mode {
            Mode::UpBuf => {
                if !endev.edev.is_cut
                    || (endev.edev.kind == ChipKind::UltrascalePlus && endev.edev.is_cut_d)
                    || vrf.find_bel_delta(bel, 0, 5, bels::BRAM_F).is_some()
                {
                    vrf.claim_node(&[bel.fwire_far(opin)]);
                    vrf.claim_pip(bel.crd(), bel.wire_far(opin), bel.wire(opin));
                }
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bels::BRAM_F) {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire_far(opin)]);
                } else if !is_cut_d(endev, bel.die, bel.row) {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::Up => {
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, bels::BRAM_F) {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
                } else if !is_cut_d(endev, bel.die, bel.row) {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::DownBuf => {
                if !endev.edev.is_cut
                    || (endev.edev.kind == ChipKind::UltrascalePlus && endev.edev.is_cut_d)
                    || vrf.find_bel_delta(bel, 0, -5, bels::BRAM_F).is_some()
                {
                    vrf.claim_node(&[bel.fwire_far(opin)]);
                    vrf.claim_pip(bel.crd(), bel.wire_far(opin), bel.wire(opin));
                }
                if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, bels::BRAM_F) {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire_far(opin)]);
                } else if !is_cut_u(endev, bel.die, bel.row) {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::DownHalfReg => match bel.row.to_idx() % Chip::ROWS_PER_REG {
                25 => (),
                55 => {
                    if !(endev.edev.kind == ChipKind::UltrascalePlus
                        && endev.edev.is_cut_d
                        && is_cut_u(endev, bel.die, bel.row))
                    {
                        vrf.claim_node(&[bel.fwire_far(ipin)]);
                    }
                }
                _ => {
                    if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, bels::BRAM_F) {
                        vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
                    } else {
                        vrf.claim_node(&[bel.fwire_far(ipin)]);
                    }
                }
            },
        }
    }
    vrf.verify_bel(bel, "RAMBFIFO36", &pins, &[]);
}

fn verify_bram_h(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let (kind, ul) = match bel.slot {
        bels::BRAM_H0 => ("RAMBFIFO18", 'L'),
        bels::BRAM_H1 => ("RAMB181", 'U'),
        _ => unreachable!(),
    };
    let mut pins = vec![];
    if ul == 'L' {
        pins.extend([
            ("CASPRVEMPTY".to_string(), SitePinDir::In),
            ("CASNXTEMPTY".to_string(), SitePinDir::Out),
            ("CASPRVRDEN".to_string(), SitePinDir::Out),
            ("CASNXTRDEN".to_string(), SitePinDir::In),
        ]);
    }
    for ab in ['A', 'B'] {
        for i in 0..16 {
            pins.push((format!("CASDI{ab}{ul}{i}"), SitePinDir::In));
            pins.push((format!("CASDO{ab}{ul}{i}"), SitePinDir::Out));
        }
        for i in 0..2 {
            pins.push((format!("CASDIP{ab}{ul}{i}"), SitePinDir::In));
            pins.push((format!("CASDOP{ab}{ul}{i}"), SitePinDir::Out));
        }
    }
    let pin_refs: Vec<_> = pins.iter().map(|&(ref pin, dir)| (&pin[..], dir)).collect();
    vrf.verify_bel(bel, kind, &pin_refs, &[]);
    let obel = vrf.find_bel_sibling(bel, bels::BRAM_F);
    for (pin, dir) in pin_refs {
        vrf.claim_node(&[bel.fwire(pin)]);
        match dir {
            SitePinDir::In => vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire_far(pin)),
            SitePinDir::Out => vrf.claim_pip(bel.crd(), obel.wire_far(pin), bel.wire(pin)),
            _ => unreachable!(),
        }
    }
}

fn verify_uram(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    for ab in ['A', 'B'] {
        for i in 0..23 {
            pairs.push((
                format!("CAS_IN_ADDR_{ab}{i}"),
                format!("CAS_OUT_ADDR_{ab}{i}"),
            ));
        }
        for i in 0..9 {
            pairs.push((
                format!("CAS_IN_BWE_{ab}{i}"),
                format!("CAS_OUT_BWE_{ab}{i}"),
            ));
        }
        for i in 0..72 {
            pairs.push((
                format!("CAS_IN_DIN_{ab}{i}"),
                format!("CAS_OUT_DIN_{ab}{i}"),
            ));
            pairs.push((
                format!("CAS_IN_DOUT_{ab}{i}"),
                format!("CAS_OUT_DOUT_{ab}{i}"),
            ));
        }
        for pin in ["EN", "RDACCESS", "RDB_WR", "DBITERR", "SBITERR"] {
            pairs.push((format!("CAS_IN_{pin}_{ab}"), format!("CAS_OUT_{pin}_{ab}")));
        }
    }
    let mut pins = vec![];
    for (ipin, opin) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        if bel.slot == bels::URAM0 {
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -15, bels::URAM3) {
                vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            } else if !is_cut_d(endev, bel.die, bel.row) {
                vrf.claim_node(&[bel.fwire_far(ipin)]);
            }
        } else {
            let oslot = match bel.slot {
                bels::URAM1 => bels::URAM0,
                bels::URAM2 => bels::URAM1,
                bels::URAM3 => bels::URAM2,
                _ => unreachable!(),
            };
            let obel = vrf.find_bel_sibling(bel, oslot);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_bel(bel, "URAM288", &pins, &[]);
}

fn verify_laguna(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let skip = if bel.row.to_idx() < Chip::ROWS_PER_REG {
        bel.die.to_idx() == 0
    } else {
        bel.die.to_idx() == endev.edev.chips.len() - 1
    };
    if !skip {
        vrf.verify_bel(
            bel,
            "LAGUNA",
            &[
                ("RXD0", SitePinDir::In),
                ("RXD1", SitePinDir::In),
                ("RXD2", SitePinDir::In),
                ("RXD3", SitePinDir::In),
                ("RXD4", SitePinDir::In),
                ("RXD5", SitePinDir::In),
                ("RXQ0", SitePinDir::Out),
                ("RXQ1", SitePinDir::Out),
                ("RXQ2", SitePinDir::Out),
                ("RXQ3", SitePinDir::Out),
                ("RXQ4", SitePinDir::Out),
                ("RXQ5", SitePinDir::Out),
                ("TXQ0", SitePinDir::Out),
                ("TXQ1", SitePinDir::Out),
                ("TXQ2", SitePinDir::Out),
                ("TXQ3", SitePinDir::Out),
                ("TXQ4", SitePinDir::Out),
                ("TXQ5", SitePinDir::Out),
            ],
            &["RXOUT0", "RXOUT1", "RXOUT2", "RXOUT3", "RXOUT4", "RXOUT5"],
        );
    }
    let bel_vcc = vrf.find_bel_sibling(bel, bels::VCC_LAGUNA);
    let mut obel = None;

    if bel.row.to_idx() < Chip::ROWS_PER_REG && !skip {
        let odie = bel.die - 1;
        let orow = RowId::from_idx(
            endev.edev.egrid.die(odie).rows().len() - Chip::ROWS_PER_REG + bel.row.to_idx(),
        );
        obel = vrf.find_bel(CellCoord::new(odie, bel.col, orow).bel(bel.slot));
        assert!(obel.is_some());
    }
    for i in 0..6 {
        vrf.claim_node(&[bel.fwire(&format!("TXQ{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("TXOUT{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("RXD{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("RXQ{i}"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("TXOUT{i}")),
            bel.wire(&format!("TXD{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("TXOUT{i}")),
            bel.wire(&format!("TXQ{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXOUT{i}")),
            bel.wire(&format!("RXD{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXOUT{i}")),
            bel.wire(&format!("RXQ{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXD{i}")),
            bel.wire(&format!("TXOUT{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXD{i}")),
            bel.wire(&format!("UBUMP{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("UBUMP{i}")),
            bel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("UBUMP{i}")),
            bel.wire(&format!("TXOUT{i}")),
        );
        if let Some(ref obel) = obel {
            vrf.claim_node(&[
                bel.fwire(&format!("UBUMP{i}")),
                obel.fwire(&format!("UBUMP{i}")),
            ]);
        } else if skip {
            vrf.claim_node(&[bel.fwire(&format!("UBUMP{i}"))]);
        }
    }
}

fn verify_laguna_extra(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let skip = if bel.row.to_idx() < Chip::ROWS_PER_REG {
        bel.die.to_idx() == 0
    } else {
        bel.die.to_idx() == endev.edev.chips.len() - 1
    };
    let bel_vcc = vrf.find_bel_sibling(bel, bels::VCC_LAGUNA);
    let mut obel = None;

    if bel.row.to_idx() < Chip::ROWS_PER_REG && !skip {
        let odie = bel.die - 1;
        let orow = RowId::from_idx(
            endev.edev.egrid.die(odie).rows().len() - Chip::ROWS_PER_REG + bel.row.to_idx(),
        );
        obel = vrf.find_bel(CellCoord::new(odie, bel.col, orow).bel(bel.slot));
        assert!(obel.is_some());
    }

    if !endev.edev.is_cut {
        vrf.claim_node(&[bel.fwire("RXD")]);
        vrf.claim_pip(bel.crd(), bel.wire("RXD"), bel.wire("TXOUT"));
        vrf.claim_pip(bel.crd(), bel.wire("RXD"), bel.wire("UBUMP"));
    }
    vrf.claim_node(&[bel.fwire("TXOUT")]);
    vrf.claim_pip(bel.crd(), bel.wire("UBUMP"), bel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), bel.wire("UBUMP"), bel.wire("TXOUT"));
    if let Some(ref obel) = obel {
        vrf.claim_node(&[bel.fwire("UBUMP"), obel.fwire("UBUMP")]);
    } else if skip {
        vrf.claim_node(&[bel.fwire("UBUMP")]);
    }
}

fn verify_vcc(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("VCC")]);
}

fn verify_pcie(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let reg = chip.row_to_reg(bel.row);
    if endev
        .edev
        .disabled
        .contains(&DisabledPart::HardIp(bel.die, bel.col, reg))
    {
        return;
    }
    let kind = match bel.slot {
        bels::PCIE3 => "PCIE_3_1",
        bels::PCIE4 => "PCIE40E4",
        bels::PCIE4C => "PCIE4CE4",
        bels::PCIE4CE => "PCIE4CE",
        _ => unreachable!(),
    };
    vrf.verify_bel(
        bel,
        kind,
        &[
            ("MCAP_PERST0_B", SitePinDir::In),
            ("MCAP_PERST1_B", SitePinDir::In),
        ],
        &[],
    );
    if bel.wire("MCAP_PERST0_B") != bel.wire_far("MCAP_PERST0_B") {
        vrf.claim_node(&[bel.fwire("MCAP_PERST0_B")]);
        vrf.claim_node(&[bel.fwire("MCAP_PERST1_B")]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("MCAP_PERST0_B"),
            bel.wire_far("MCAP_PERST0_B"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("MCAP_PERST1_B"),
            bel.wire_far("MCAP_PERST1_B"),
        );
    }
    if vrf
        .find_bel_delta(bel, 0, Chip::ROWS_PER_REG as isize, bels::CFG)
        .is_some()
        && !endev.edev.chips[bel.die].is_nocfg()
    {
        if endev.edev.chips[bel.die].config_kind.is_csec() {
            // nothingness
        } else {
            let obel = get_hpiob_bel(
                endev,
                vrf,
                *endev.edev.cfg_io[bel.die]
                    .get_by_left(&SharedCfgPad::PerstN0)
                    .unwrap(),
            );
            vrf.verify_node(&[bel.fwire_far("MCAP_PERST0_B"), obel.fwire("DOUT")]);
            let obel = get_hpiob_bel(
                endev,
                vrf,
                *endev.edev.cfg_io[bel.die]
                    .get_by_left(&match endev.edev.kind {
                        ChipKind::Ultrascale => SharedCfgPad::PerstN1,
                        ChipKind::UltrascalePlus => SharedCfgPad::I2cSda,
                    })
                    .unwrap(),
            );
            vrf.verify_node(&[bel.fwire_far("MCAP_PERST1_B"), obel.fwire("DOUT")]);
        }
    } else {
        vrf.claim_node(&[bel.fwire_far("MCAP_PERST0_B")]);
        vrf.claim_node(&[bel.fwire_far("MCAP_PERST1_B")]);
    }
}

fn verify_cmac(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let reg = chip.row_to_reg(bel.row);
    if endev
        .edev
        .disabled
        .contains(&DisabledPart::HardIp(bel.die, bel.col, reg))
    {
        return;
    }
    vrf.verify_bel(
        bel,
        if endev.edev.kind == ChipKind::Ultrascale {
            "CMAC_SITE"
        } else {
            "CMACE4"
        },
        &[],
        &[],
    );
}
fn verify_ilkn(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let reg = chip.row_to_reg(bel.row);
    if endev
        .edev
        .disabled
        .contains(&DisabledPart::HardIp(bel.die, bel.col, reg))
    {
        return;
    }
    vrf.verify_bel(
        bel,
        if endev.edev.kind == ChipKind::Ultrascale {
            "ILKN_SITE"
        } else {
            "ILKNE4"
        },
        &[],
        &[],
    );
}

fn verify_ps(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins_clk = [
        (0, 0, "O_DBG_L0_TXCLK"),
        (0, 1, "O_DBG_L0_RXCLK"),
        (0, 2, "O_DBG_L1_TXCLK"),
        (0, 3, "O_DBG_L1_RXCLK"),
        (0, 4, "O_DBG_L2_TXCLK"),
        (0, 5, "O_DBG_L2_RXCLK"),
        (0, 6, "O_DBG_L3_TXCLK"),
        (0, 7, "O_DBG_L3_RXCLK"),
        (1, 0, "APLL_TEST_CLK_OUT0"),
        (1, 1, "APLL_TEST_CLK_OUT1"),
        (1, 2, "DPLL_TEST_CLK_OUT0"),
        (1, 3, "DPLL_TEST_CLK_OUT1"),
        (1, 4, "VPLL_TEST_CLK_OUT0"),
        (1, 5, "VPLL_TEST_CLK_OUT1"),
        (1, 6, "DP_AUDIO_REF_CLK"),
        (1, 7, "DP_VIDEO_REF_CLK"),
        (1, 8, "DDR_DTO0"),
        (1, 9, "DDR_DTO1"),
        (2, 0, "PL_CLK0"),
        (2, 1, "PL_CLK1"),
        (2, 2, "PL_CLK2"),
        (2, 3, "PL_CLK3"),
        (2, 4, "IOPLL_TEST_CLK_OUT0"),
        (2, 5, "IOPLL_TEST_CLK_OUT1"),
        (2, 6, "RPLL_TEST_CLK_OUT0"),
        (2, 7, "RPLL_TEST_CLK_OUT1"),
        (2, 8, "FMIO_GEM0_FIFO_TX_CLK_TO_PL_BUFG"),
        (2, 9, "FMIO_GEM0_FIFO_RX_CLK_TO_PL_BUFG"),
        (2, 10, "FMIO_GEM1_FIFO_TX_CLK_TO_PL_BUFG"),
        (2, 11, "FMIO_GEM1_FIFO_RX_CLK_TO_PL_BUFG"),
        (2, 12, "FMIO_GEM2_FIFO_TX_CLK_TO_PL_BUFG"),
        (2, 13, "FMIO_GEM2_FIFO_RX_CLK_TO_PL_BUFG"),
        (2, 14, "FMIO_GEM3_FIFO_TX_CLK_TO_PL_BUFG"),
        (2, 15, "FMIO_GEM3_FIFO_RX_CLK_TO_PL_BUFG"),
        (2, 16, "FMIO_GEM_TSU_CLK_TO_PL_BUFG"),
        (2, 17, "PS_PL_SYSOSC_CLK"),
    ];
    let pins_cfg_in = [
        "BSCAN_RESET_TAP_B",
        "BSCAN_CLOCKDR",
        "BSCAN_SHIFTDR",
        "BSCAN_UPDATEDR",
        "BSCAN_INTEST",
        "BSCAN_EXTEST",
        "BSCAN_INIT_MEMORY",
        "BSCAN_AC_TEST",
        "BSCAN_AC_MODE",
        "BSCAN_MISR_JTAG_LOAD",
        "PSS_CFG_RESET_B",
        "PSS_FST_CFG_B",
        "PSS_GTS_CFG_B",
        "PSS_GTS_USR_B",
        "PSS_GHIGH_B",
        "PSS_GPWRDWN_B",
        "PCFG_POR_B",
    ];
    let mut pins_dummy_in = vec![
        "IDCODE15",
        "IDCODE17",
        "IDCODE18",
        "IDCODE20",
        "IDCODE21",
        "IDCODE28",
        "IDCODE29",
        "IDCODE30",
        "IDCODE31",
        "PS_VERSION_0",
        "PS_VERSION_2",
        "PS_VERSION_3",
    ];
    let tk = vrf.rd.tile_kinds.get("PSS_ALTO").unwrap().1;
    let site = tk.sites.values().next().unwrap();
    if site.pins.contains_key("IDCODE16") {
        pins_dummy_in.push("IDCODE16");
    }
    let mut pins = vec![];
    let obels = [
        vrf.find_bel_delta(bel, 0, 30, bels::RCLK_PS).unwrap(),
        vrf.find_bel_delta(bel, 0, 90, bels::RCLK_PS).unwrap(),
        vrf.find_bel_delta(bel, 0, 150, bels::RCLK_PS).unwrap(),
    ];
    for (reg, idx, pin) in pins_clk {
        vrf.claim_node(&[
            bel.fwire(pin),
            obels[reg].fwire(&format!("PS_TO_PL_CLK{idx}")),
        ]);
        pins.push((pin, SitePinDir::Out));
    }
    for pin in pins_cfg_in {
        vrf.claim_node(&[bel.fwire(pin)]);
        pins.push((pin, SitePinDir::In));
    }
    for &pin in &pins_dummy_in {
        pins.push((pin, SitePinDir::In));
    }
    if !endev.edev.disabled.contains(&DisabledPart::Ps) {
        vrf.verify_bel_dummies(bel, "PS8", &pins, &[], &pins_dummy_in);
    }
}

fn verify_vcu(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins_clk = [(0, "VCU_PLL_TEST_CLK_OUT0"), (1, "VCU_PLL_TEST_CLK_OUT1")];
    let obel = vrf.find_bel_delta(bel, 0, 30, bels::RCLK_PS).unwrap();
    let mut pins = vec![];
    for (idx, pin) in pins_clk {
        vrf.claim_node(&[bel.fwire(pin), obel.fwire(&format!("PS_TO_PL_CLK{idx}"))]);
        pins.push((pin, SitePinDir::Out));
    }
    if !endev.edev.disabled.contains(&DisabledPart::Vcu) {
        vrf.verify_bel(bel, "VCU", &pins, &[]);
    }
}

fn verify_sysmon(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let vaux: Vec<_> = (0..16)
        .map(|i| (format!("VP_AUX{i}"), format!("VN_AUX{i}")))
        .collect();
    let mut pins = vec![];
    if endev.edev.kind == ChipKind::Ultrascale {
        pins.extend([
            ("I2C_SCLK_IN", SitePinDir::In),
            ("I2C_SCLK_TS", SitePinDir::Out),
            ("I2C_SDA_IN", SitePinDir::In),
            ("I2C_SDA_TS", SitePinDir::Out),
        ]);
        let obel = get_hpiob_bel(
            endev,
            vrf,
            *endev.edev.cfg_io[bel.die]
                .get_by_left(&SharedCfgPad::I2cSclk)
                .unwrap(),
        );
        vrf.verify_node(&[bel.fwire("I2C_SCLK_TS"), obel.fwire_far("TSDI")]);
        vrf.verify_node(&[bel.fwire_far("I2C_SCLK_IN"), obel.fwire("DOUT")]);
        let obel = get_hpiob_bel(
            endev,
            vrf,
            *endev.edev.cfg_io[bel.die]
                .get_by_left(&SharedCfgPad::I2cSda)
                .unwrap(),
        );
        vrf.verify_node(&[bel.fwire("I2C_SDA_TS"), obel.fwire_far("TSDI")]);
        vrf.verify_node(&[bel.fwire_far("I2C_SDA_IN"), obel.fwire("DOUT")]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I2C_SCLK_IN"),
            bel.wire_far("I2C_SCLK_IN"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("I2C_SDA_IN"),
            bel.wire_far("I2C_SDA_IN"),
        );
    }
    for (vp, vn) in &vaux {
        pins.extend([(&vp[..], SitePinDir::In), (&vn[..], SitePinDir::In)]);
    }
    let kind = match endev.edev.kind {
        ChipKind::Ultrascale => "SYSMONE1",
        ChipKind::UltrascalePlus => "SYSMONE4",
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    if endev.edev.kind == ChipKind::UltrascalePlus {
        for i in 0..16 {
            for pn in ['P', 'N'] {
                let pin = format!("V{pn}_AUX{i}");
                vrf.claim_node(&[bel.fwire_far(&pin)]);
                vrf.claim_pip(bel.crd(), bel.wire(&pin), bel.wire_far(&pin));
            }
        }
    }
}

fn verify_abus_switch(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let reg = chip.row_to_reg(bel.row);
    let mut pins = &[][..];
    if endev.edev.kind == ChipKind::UltrascalePlus
        && !bel.info.pins.contains_key("TEST_ANALOGBUS_SEL_B")
    {
        pins = &[("TEST_ANALOGBUS_SEL_B", SitePinDir::In)];
    }
    let mut skip = false;
    if bel.tcls.starts_with("GTM") {
        skip = endev
            .edev
            .disabled
            .contains(&DisabledPart::Gt(bel.die, bel.col, reg));
    }
    if endev
        .edev
        .disabled
        .contains(&DisabledPart::GtBufs(bel.die, bel.col, reg))
    {
        skip = true;
    }

    if !skip {
        vrf.verify_bel(bel, "ABUS_SWITCH", pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_bufce_leaf_x16(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let clk_in: [_; 16] = core::array::from_fn(|i| format!("CLK_IN{i}"));
    let pins: Vec<_> = clk_in.iter().map(|x| (&x[..], SitePinDir::In)).collect();
    vrf.verify_bel(bel, "BUFCE_LEAF_X16", &pins, &[]);
    let obel = vrf.find_bel_sibling(bel, bels::RCLK_INT_CLK);
    for pin in &clk_in {
        vrf.claim_node(&[bel.fwire(pin)]);
        for j in 0..24 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(&format!("HDISTR{j}")));
        }
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("VCC"));
    }
}

fn verify_bufce_leaf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![
        ("CLK_CASC_OUT", SitePinDir::Out),
        ("CLK_IN", SitePinDir::In),
    ];
    if bel.slot != bels::BUFCE_LEAF_S0 {
        pins.push(("CLK_CASC_IN", SitePinDir::In));
    }
    vrf.verify_bel(bel, "BUFCE_LEAF", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, bels::RCLK_INT_CLK);
    for j in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN"),
            obel.wire(&format!("HDISTR{j}")),
        );
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("VCC"));
    let idx = bels::BUFCE_LEAF
        .into_iter()
        .position(|x| x == bel.slot)
        .unwrap();
    if idx != 0 {
        let oslot = bels::BUFCE_LEAF[idx - 1];
        let obel = vrf.find_bel_sibling(bel, oslot);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_CASC_IN"),
            obel.wire("CLK_CASC_OUT"),
        );
    }
}

fn find_hdistr_src<'a>(
    endev: &ExpandedNamedDevice,
    vrf: &Verifier<'a>,
    cell: CellCoord,
) -> BelContext<'a> {
    let src = endev.edev.hdistr_src[cell.col];
    match src {
        ClkSrc::Gt(scol) => vrf
            .find_bel(cell.with_col(scol).bel(bels::RCLK_GT))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::RCLK_XIPHY)))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::CMT)))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::CMTXP)))
            .unwrap(),
        ClkSrc::DspSplitter(scol) => vrf
            .find_bel(cell.with_col(scol).bel(bels::RCLK_SPLITTER))
            .unwrap(),
        ClkSrc::Cmt(scol) => vrf
            .find_bel(cell.with_col(scol).bel(bels::RCLK_XIPHY))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::CMT)))
            .unwrap(),
        ClkSrc::RouteSplitter(_) => unreachable!(),
        ClkSrc::RightHdio(scol) => vrf
            .find_bel(cell.with_col(scol).bel(bels::RCLK_HDIO))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::RCLK_HDIOL)))
            .unwrap(),
    }
}

fn find_hroute_src<'a>(
    endev: &ExpandedNamedDevice,
    vrf: &Verifier<'a>,
    cell: CellCoord,
) -> BelContext<'a> {
    let src = endev.edev.hroute_src[cell.col];
    match src {
        ClkSrc::Gt(scol) => vrf
            .find_bel(cell.with_col(scol).bel(bels::RCLK_GT))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::CMT)))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::CMTXP)))
            .unwrap(),
        ClkSrc::DspSplitter(scol) => vrf
            .find_bel(cell.with_col(scol).bel(bels::RCLK_SPLITTER))
            .unwrap(),
        ClkSrc::Cmt(scol) => vrf.find_bel(cell.with_col(scol).bel(bels::CMT)).unwrap(),
        ClkSrc::RouteSplitter(scol) => vrf
            .find_bel(cell.with_col(scol).bel(bels::RCLK_HROUTE_SPLITTER))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::RCLK_HDIO)))
            .unwrap(),
        ClkSrc::RightHdio(scol) => vrf
            .find_bel(cell.with_col(scol).bel(bels::RCLK_HDIO))
            .or_else(|| vrf.find_bel(cell.with_col(scol).bel(bels::RCLK_HDIOL)))
            .unwrap(),
    }
}

fn verify_rclk_int(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = find_hdistr_src(endev, vrf, bel.cell);
    for i in 0..24 {
        vrf.verify_node(&[
            bel.fwire(&format!("HDISTR{i}")),
            obel.fwire(&format!("HDISTR{i}_L")),
        ]);
    }
}

fn verify_rclk_splitter(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_RCLK_SPLITTER);
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            bel.wire(&format!("HDISTR{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            bel.wire(&format!("HDISTR{i}_L")),
        );
    }
    let obel_hd = find_hdistr_src(endev, vrf, bel.cell);
    let obel_hr = find_hroute_src(endev, vrf, bel.cell);
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        vrf.verify_node(&[
            bel.fwire(&format!("HDISTR{i}_R")),
            obel_hd.fwire(&format!("HDISTR{i}_L")),
        ]);
        vrf.verify_node(&[
            bel.fwire(&format!("HROUTE{i}_R")),
            obel_hr.fwire(&format!("HROUTE{i}_L")),
        ]);
    }
}

fn verify_rclk_hroute_splitter(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    bel: &BelContext<'_>,
) {
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_RCLK_HROUTE_SPLITTER);
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
    }
    let obel_hr = find_hroute_src(endev, vrf, bel.cell);
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        vrf.verify_node(&[
            bel.fwire(&format!("HROUTE{i}_R")),
            obel_hr.fwire(&format!("HROUTE{i}_L")),
        ]);
    }
}

fn verify_bufce_row(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let pins = vec![
        ("CLK_IN", SitePinDir::In),
        ("CLK_OUT", SitePinDir::Out),
        ("CLK_OUT_OPT_DLY", SitePinDir::Out),
    ];
    let is_io = matches!(chip.columns[bel.col].kind, ColumnKind::Io(_));
    let kind = if endev.edev.kind == ChipKind::UltrascalePlus && !is_io {
        "BUFCE_ROW_FSR"
    } else {
        "BUFCE_ROW"
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    if is_io {
        let idx = bels::BUFCE_ROW_CMT
            .into_iter()
            .position(|x| x == bel.slot)
            .unwrap();

        let obel_cmt = vrf
            .find_bel_delta(bel, 0, 0, bels::CMT)
            .or_else(|| vrf.find_bel_delta(bel, 0, 0, bels::CMTXP))
            .unwrap();
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN"),
            obel_cmt.wire(&format!("VDISTR{idx}_B")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN"),
            obel_cmt.wire(&format!("VDISTR{idx}_T")),
        );
    } else {
        let idx = bels::BUFCE_ROW_RCLK
            .into_iter()
            .position(|x| x == bel.slot)
            .unwrap();

        let hidx = chip.columns[bel.col].clk[idx];

        let obel_gtb = vrf.find_bel_sibling(bel, bels::GCLK_TEST_BUF_RCLK[idx]);
        if let Some(hidx) = hidx {
            let obel_hd = find_hdistr_src(endev, vrf, bel.cell);
            let obel_hr = find_hroute_src(endev, vrf, bel.cell);
            vrf.verify_node(&[
                obel_gtb.fwire_far("CLK_IN"),
                obel_hd.fwire(&format!("HDISTR{hidx}_L")),
            ]);
            vrf.verify_node(&[
                bel.fwire("HROUTE"),
                obel_hr.fwire(&format!("HROUTE{hidx}_L")),
            ]);
        } else {
            vrf.claim_node(&[obel_gtb.fwire_far("CLK_IN")]);
            vrf.claim_node(&[bel.fwire("HROUTE")]);
        }

        vrf.claim_node(&[bel.fwire("VROUTE_T")]);
        vrf.claim_node(&[bel.fwire("VDISTR_T")]);
        let obel_s = vrf
            .find_bel_delta(bel, 0, -(Chip::ROWS_PER_REG as isize), bel.slot)
            .or_else(|| {
                if bel.die.to_idx() == 0 || bel.row.to_idx() != 30 || hidx.is_none() {
                    return None;
                }
                let odie = bel.die - 1;
                let ogrid = endev.edev.chips[odie];
                vrf.find_bel(
                    CellCoord::new(
                        odie,
                        bel.col,
                        ogrid.row_reg_rclk(ogrid.regs().next_back().unwrap()),
                    )
                    .bel(bel.slot),
                )
            });
        if let Some(obel) = obel_s {
            vrf.verify_node(&[bel.fwire("VROUTE_B"), obel.fwire("VROUTE_T")]);
            vrf.verify_node(&[bel.fwire("VDISTR_B"), obel.fwire("VDISTR_T")]);
        } else {
            vrf.claim_node(&[bel.fwire("VROUTE_B")]);
            vrf.claim_node(&[bel.fwire("VDISTR_B")]);
        }

        let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_RCLK_V);
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B"), bel.wire("VDISTR_B_MUX"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T"), bel.wire("VDISTR_T_MUX"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B"), bel.wire("VROUTE_B_MUX"));
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T"), bel.wire("VROUTE_T_MUX"));
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("HROUTE"), bel.wire("HROUTE_MUX"));
        vrf.claim_pip(bel.crd(), bel.wire("HROUTE"), obel_vcc.wire("VCC"));

        vrf.claim_node(&[bel.fwire("HROUTE_MUX")]);
        vrf.claim_node(&[bel.fwire("VROUTE_B_MUX")]);
        vrf.claim_node(&[bel.fwire("VROUTE_T_MUX")]);
        vrf.claim_node(&[bel.fwire("VDISTR_B_MUX")]);
        vrf.claim_node(&[bel.fwire("VDISTR_T_MUX")]);

        if endev.edev.kind == ChipKind::Ultrascale {
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("VDISTR_B"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("VDISTR_T"));

            vrf.claim_pip(bel.crd(), bel.wire("HROUTE_MUX"), bel.wire("VROUTE_B"));
            vrf.claim_pip(bel.crd(), bel.wire("HROUTE_MUX"), bel.wire("VROUTE_T"));

            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_MUX"), bel.wire("VDISTR_T"));
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_MUX"), bel.wire("VROUTE_T"));
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_MUX"), bel.wire("HROUTE"));
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_MUX"), bel.wire("VDISTR_B"));
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_MUX"), bel.wire("VROUTE_B"));
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_MUX"), bel.wire("HROUTE"));

            vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B_MUX"), bel.wire("VROUTE_T"));
            vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B_MUX"), bel.wire("HROUTE"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("VROUTE_B_MUX"),
                obel_gtb.wire("CLK_OUT"),
            );
            vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T_MUX"), bel.wire("VROUTE_B"));
            vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T_MUX"), bel.wire("HROUTE"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("VROUTE_T_MUX"),
                obel_gtb.wire("CLK_OUT"),
            );
        } else {
            vrf.claim_node(&[bel.fwire("VDISTR_B_BUF")]);
            vrf.claim_node(&[bel.fwire("VDISTR_T_BUF")]);
            vrf.claim_node(&[bel.fwire("VROUTE_B_BUF")]);
            vrf.claim_node(&[bel.fwire("VROUTE_T_BUF")]);
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_BUF"), bel.wire("VDISTR_B"));
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_BUF"), bel.wire("VDISTR_T"));
            vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B_BUF"), bel.wire("VROUTE_B"));
            vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T_BUF"), bel.wire("VROUTE_T"));

            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("VDISTR_B_BUF"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("VDISTR_T_BUF"));

            vrf.claim_pip(bel.crd(), bel.wire("HROUTE_MUX"), bel.wire("VROUTE_B_BUF"));
            vrf.claim_pip(bel.crd(), bel.wire("HROUTE_MUX"), bel.wire("VROUTE_T_BUF"));

            vrf.claim_pip(
                bel.crd(),
                bel.wire("VDISTR_B_MUX"),
                bel.wire("VDISTR_T_BUF"),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("VDISTR_B_MUX"),
                bel.wire("VROUTE_T_BUF"),
            );
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_MUX"), bel.wire("HROUTE"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("VDISTR_T_MUX"),
                bel.wire("VDISTR_B_BUF"),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("VDISTR_T_MUX"),
                bel.wire("VROUTE_B_BUF"),
            );
            vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_MUX"), bel.wire("HROUTE"));

            vrf.claim_pip(
                bel.crd(),
                bel.wire("VROUTE_B_MUX"),
                bel.wire("VROUTE_T_BUF"),
            );
            vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B_MUX"), bel.wire("HROUTE"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("VROUTE_B_MUX"),
                obel_gtb.wire("CLK_OUT"),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("VROUTE_T_MUX"),
                bel.wire("VROUTE_B_BUF"),
            );
            vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T_MUX"), bel.wire("HROUTE"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("VROUTE_T_MUX"),
                obel_gtb.wire("CLK_OUT"),
            );
        }

        vrf.claim_pip(bel.crd(), obel_gtb.wire_far("CLK_IN"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), obel_gtb.wire_far("CLK_IN"), bel.wire("CLK_OUT"));
    }
}

fn verify_gclk_test_buf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("CLK_IN", SitePinDir::In), ("CLK_OUT", SitePinDir::Out)];
    vrf.verify_bel(bel, "GCLK_TEST_BUFE3", &pins, &[]);
    if !bel.naming.pins["CLK_IN"].pips.is_empty() {
        vrf.claim_node(&[bel.fwire("CLK_IN")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire_far("CLK_IN"));
    }
    vrf.claim_node(&[bel.fwire("CLK_OUT")]);
    // other stuff dealt with in BUFCE_ROW
}

fn verify_bufg_ps(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("CLK_OUT", SitePinDir::Out), ("CLK_IN", SitePinDir::In)];
    vrf.verify_bel(bel, "BUFG_PS", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_RCLK_PS);
    let obel = vrf.find_bel_sibling(bel, bels::RCLK_PS);
    for j in 0..18 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN"),
            obel.wire(&format!("PS_TO_PL_CLK{j}")),
        );
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CKINT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("CLK_IN_DUMMY"));

    vrf.claim_node(&[bel.fwire("CLK_IN_DUMMY")]);
}

fn verify_rclk_ps(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_RCLK_PS);
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}")),
            obel_vcc.wire("VCC"),
        );
        let obel = vrf.find_bel_sibling(bel, bels::BUFG_PS[i]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}")),
            obel.wire("CLK_OUT"),
        );
    }
    let obel_hr = find_hroute_src(endev, vrf, bel.cell);
    for i in 0..24 {
        vrf.verify_node(&[
            bel.fwire(&format!("HROUTE{i}")),
            obel_hr.fwire(&format!("HROUTE{i}_L")),
        ]);
    }
}

fn verify_hdiob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::HDIOB.into_iter().position(|x| bel.slot == x).unwrap();
    let is_b = bel.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG);
    let is_m = idx.is_multiple_of(2);
    let chip = endev.edev.chips[bel.die];
    let is_hdiolc = chip.config_kind.is_csec();
    let reg = chip.row_to_reg(bel.row);
    let hid = TileIobId::from_idx(
        idx + if is_b {
            0
        } else if is_hdiolc {
            42
        } else {
            12
        },
    );
    let iocrd = if is_hdiolc {
        IoCoord::HdioLc(HdioCoord {
            die: bel.die,
            col: bel.col,
            reg,
            iob: hid,
        })
    } else {
        IoCoord::Hdio(HdioCoord {
            die: bel.die,
            col: bel.col,
            reg,
            iob: hid,
        })
    };
    let kind = if is_m { "HDIOB_M" } else { "HDIOB_S" };
    let pins = [
        ("OP", SitePinDir::In),
        ("TSP", SitePinDir::In),
        ("O_B", SitePinDir::Out),
        ("TSTATEB", SitePinDir::Out),
        ("OUTB_B", SitePinDir::Out),
        ("OUTB_B_IN", SitePinDir::In),
        ("TSTATE_OUT", SitePinDir::Out),
        ("TSTATE_IN", SitePinDir::In),
        ("LVDS_TRUE", SitePinDir::In),
        ("PAD_RES", SitePinDir::Out),
        ("I", SitePinDir::Out),
        ("IO", SitePinDir::In),
        ("SWITCH_OUT", SitePinDir::Out),
    ];
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::HdioIob(bel.die, bel.col, reg, hid))
    {
        vrf.verify_bel_dummies(bel, kind, &pins, &[], &["IO"]);
    }
    for (pin, _) in pins {
        if pin != "IO" && pin != "SWITCH_OUT" {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
    let obel = vrf.find_bel_sibling(bel, bels::HDIOB[idx ^ 1]);
    vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), obel.wire("OUTB_B"));
    vrf.claim_pip(bel.crd(), bel.wire("TSTATE_IN"), obel.wire("TSTATE_OUT"));

    let io_info = endev.edev.get_io_info(iocrd);
    if chip.config_kind.is_csec() {
        if let Some(ams_idx) = io_info.sm_pair {
            let scol = chip.col_cfg();
            let srow = chip.row_ams();
            let obel = vrf.get_bel(bel.cell.with_cr(scol, srow).bel(bels::SYSMON));
            vrf.verify_node(&[
                bel.fwire("SWITCH_OUT"),
                obel.fwire_far(&format!(
                    "V{pn}_AUX{ams_idx}",
                    pn = if is_m { 'P' } else { 'N' }
                )),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire("SWITCH_OUT")]);
        }
    } else {
        vrf.claim_node(&[bel.fwire("SWITCH_OUT")]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("SWITCH_OUT"),
            bel.wire("SWITCH_OUT"),
        );
        if let Some(ams_idx) = io_info.sm_pair {
            let scol = chip.col_cfg();
            let srow = chip.row_ams();
            let obel = vrf.get_bel(bel.cell.with_cr(scol, srow).bel(bels::SYSMON));
            vrf.verify_node(&[
                bel.fwire_far("SWITCH_OUT"),
                obel.fwire_far(&format!(
                    "V{pn}_AUX{ams_idx}",
                    pn = if is_m { 'P' } else { 'N' }
                )),
            ]);
        }
    }
}

fn verify_hdiodiffin(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::HDIOB_DIFF_IN
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let is_b = bel.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG);
    let hid = TileIobId::from_idx(2 * idx + if is_b { 0 } else { 12 });
    let chip = endev.edev.chips[bel.die];
    let reg = chip.row_to_reg(bel.row);
    let pins = [
        ("LVDS_TRUE", SitePinDir::Out),
        ("LVDS_COMP", SitePinDir::Out),
        ("PAD_RES_0", SitePinDir::In),
        ("PAD_RES_1", SitePinDir::In),
    ];
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::HdioIob(bel.die, bel.col, reg, hid))
    {
        vrf.verify_bel(bel, "HDIOBDIFFINBUF", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel_m = vrf.find_bel_sibling(bel, bels::HDIOB[2 * idx]);
    let obel_s = vrf.find_bel_sibling(bel, bels::HDIOB[2 * idx + 1]);
    vrf.claim_pip(bel.crd(), obel_m.wire("LVDS_TRUE"), bel.wire("LVDS_TRUE"));
    vrf.claim_pip(bel.crd(), obel_s.wire("LVDS_TRUE"), bel.wire("LVDS_COMP"));
    vrf.claim_pip(bel.crd(), bel.wire("PAD_RES_0"), obel_m.wire("PAD_RES"));
    vrf.claim_pip(bel.crd(), bel.wire("PAD_RES_1"), obel_s.wire("PAD_RES"));
}

fn verify_hdiologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::HDIOLOGIC
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let is_m = idx.is_multiple_of(2);
    let kind = if is_m { "HDIOLOGIC_M" } else { "HDIOLOGIC_S" };
    let pins = if is_m {
        [
            ("IPFFM_D", SitePinDir::In),
            ("OPFFM_Q", SitePinDir::Out),
            ("TFFM_Q", SitePinDir::Out),
        ]
    } else {
        [
            ("IPFFS_D", SitePinDir::In),
            ("OPFFS_Q", SitePinDir::Out),
            ("TFFS_Q", SitePinDir::Out),
        ]
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, bels::HDIOB[idx]);
    vrf.claim_pip(
        bel.crd(),
        obel.wire("OP"),
        bel.wire(if is_m { "OPFFM_Q" } else { "OPFFS_Q" }),
    );
    vrf.claim_pip(
        bel.crd(),
        obel.wire("TSP"),
        bel.wire(if is_m { "TFFM_Q" } else { "TFFS_Q" }),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire(if is_m { "IPFFM_D" } else { "IPFFS_D" }),
        obel.wire("I"),
    );
}

fn verify_bufgce_hdio(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [("CLK_OUT", SitePinDir::Out), ("CLK_IN", SitePinDir::In)];
    vrf.verify_bel(bel, "BUFGCE_HDIO", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel_rclk = vrf
        .find_bel_delta(bel, 0, 0, bels::RCLK_HDIO)
        .or_else(|| vrf.find_bel_delta(bel, 0, 0, bels::RCLK_HDIOL))
        .unwrap();
    vrf.claim_node(&[bel.fwire("CLK_IN_MUX")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("CLK_IN_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel_rclk.wire("CKINT"));
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN_MUX"),
            obel_rclk.wire(&format!("CCIO{i}")),
        );
    }
}

fn verify_rclk_hdio(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let obel_bufgce: [_; 4] =
        core::array::from_fn(|i| vrf.find_bel_sibling(bel, bels::BUFGCE_HDIO[i]));
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_RCLK_HDIO);
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            obel_vcc.wire("VCC"),
        );
        for obel in &obel_bufgce {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HDISTR{i}_MUX")),
                obel.wire("CLK_OUT"),
            );
        }

        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );

        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );
    }
    if chip.columns[bel.col].kind == ColumnKind::HdioS {
        for (i, slot) in [
            (0, bels::HDIOB22),
            (1, bels::HDIOB24),
            (2, bels::HDIOB8),
            (3, bels::HDIOB10),
        ] {
            let obel = vrf.find_bel_delta(bel, 0, -30, slot).unwrap();
            vrf.verify_node(&[bel.fwire(&format!("CCIO{i}")), obel.fwire("I")]);
        }
    } else {
        for (i, dy, slot) in [
            (0, 0, bels::HDIOB0),
            (1, 0, bels::HDIOB2),
            (2, -30, bels::HDIOB8),
            (3, -30, bels::HDIOB10),
        ] {
            let obel = vrf.find_bel_delta(bel, 0, dy, slot).unwrap();
            vrf.verify_node(&[bel.fwire(&format!("CCIO{i}")), obel.fwire("I")]);
        }
    }

    if bel.col == endev.edev.chips[bel.die].columns.last_id().unwrap() {
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
            vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R"))]);
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
        }
    } else {
        let obel_hd = find_hdistr_src(endev, vrf, bel.cell.delta(1, 0));
        let obel_hr = find_hroute_src(endev, vrf, bel.cell.delta(1, 0));
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_L")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("HROUTE{i}_R")),
                obel_hr.fwire(&format!("HROUTE{i}_L")),
            ]);
        }
    }
}

fn verify_rclk_hdiol(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_bufgce: [_; 4] =
        core::array::from_fn(|i| vrf.find_bel_sibling(bel, bels::BUFGCE_HDIO[i]));
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_RCLK_HDIO);
    let lr = if endev.edev.chips[bel.die].col_side(bel.col) == DirH::W {
        'R'
    } else {
        'L'
    };
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_{lr}")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_{lr}")),
            obel_vcc.wire("VCC"),
        );
        for obel in &obel_bufgce {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HDISTR{i}_MUX")),
                obel.wire("CLK_OUT"),
            );
        }

        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );

        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );
    }
    for (i, dy, slot) in [
        (0, 0, bels::HDIOB0),
        (1, 0, bels::HDIOB2),
        (2, -30, bels::HDIOB12),
        (3, -30, bels::HDIOB10),
    ] {
        let obel = vrf.find_bel_delta(bel, 0, dy, slot).unwrap();
        vrf.verify_node(&[bel.fwire(&format!("CCIO{i}")), obel.fwire("I")]);
    }

    if endev.edev.chips[bel.die].col_side(bel.col) == DirH::E {
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R"))]);
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
        }
    } else {
        let obel_hd = find_hdistr_src(endev, vrf, bel.cell);
        let obel_cmt = vrf.find_bel_sibling(bel, bels::CMT);
        for i in 0..24 {
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_R")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("HROUTE{i}_R")),
                obel_cmt.fwire(&format!("HROUTE{i}_L")),
            ]);
        }
    }
}

fn verify_bufgce(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = endev.edev.chips[bel.die].col_side(bel.col) == DirH::W;
    let hr_lr = if is_l { 'L' } else { 'R' };
    let pins = vec![("CLK_IN", SitePinDir::In), ("CLK_OUT", SitePinDir::Out)];
    vrf.verify_bel(bel, "BUFGCE", &pins, &["CLK_IN_CKINT"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    if !bel.naming.pins["CLK_IN"].pips.is_empty() {
        vrf.claim_node(&[bel.fwire_far("CLK_IN")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire_far("CLK_IN"));
    }

    let idx = bels::BUFGCE
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();

    let obel_mmcm = vrf.find_bel_sibling(bel, bels::MMCM);
    let (is_xp, obel_cmt, obel_pll0, obel_pll1) =
        if let Some(obel) = vrf.find_bel_delta(bel, 0, 0, bels::CMT) {
            (
                false,
                obel,
                vrf.find_bel_sibling(bel, bels::PLL0),
                vrf.find_bel_sibling(bel, bels::PLL1),
            )
        } else {
            (
                true,
                vrf.find_bel_sibling(bel, bels::CMTXP),
                vrf.find_bel_sibling(bel, bels::PLLXP0),
                vrf.find_bel_sibling(bel, bels::PLLXP1),
            )
        };
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_CMT);
    let obel_gtb = vrf.find_bel_sibling(bel, bels::GCLK_TEST_BUF_CMT[idx]);
    for pin in [
        "CLKOUT0",
        "CLKOUT0B",
        "CLKOUT1",
        "CLKOUT1B",
        "CLKOUT2",
        "CLKOUT2B",
        "CLKOUT3",
        "CLKOUT3B",
        "CLKOUT4",
        "CLKOUT5",
        "CLKOUT6",
        "CLKFBOUT",
        "CLKFBOUTB",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire_far("CLK_IN"), obel_mmcm.wire(pin));
    }
    for pin in ["CLKOUT0", "CLKOUT0B", "CLKOUT1", "CLKOUT1B"] {
        vrf.claim_pip(bel.crd(), bel.wire_far("CLK_IN"), obel_pll0.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire_far("CLK_IN"), obel_pll1.wire(pin));
    }
    vrf.claim_pip(bel.crd(), bel.wire_far("CLK_IN"), obel_vcc.wire("VCC"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("CLK_IN"),
        bel.wire_far("CLK_IN_MUX_HROUTE"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("CLK_IN"),
        bel.wire_far("CLK_IN_MUX_PLL_CKINT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("CLK_IN"),
        bel.wire_far("CLK_IN_MUX_TEST"),
    );
    if !is_xp {
        for i in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("CLK_IN"),
                obel_cmt.wire(&format!("CCIO{i}")),
            );
        }
        if endev.edev.kind == ChipKind::Ultrascale {
            for i in 0..8 {
                let ii = [0, 6, 13, 19, 26, 32, 39, 45][i];
                let obel = vrf.find_bel_sibling(bel, bels::BITSLICE[ii]);
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire_far("CLK_IN"),
                    obel.wire("PHY2CLB_FIFO_WRCLK"),
                );
            }
        } else {
            for i in 0..8 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire_far("CLK_IN"),
                    obel_cmt.wire(&format!("FIFO_WRCLK{i}")),
                );
            }
        }
    } else {
        for i in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("CLK_IN"),
                obel_cmt.wire(&format!("CCIO_BOT{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("CLK_IN"),
                obel_cmt.wire(&format!("CCIO_MID{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("CLK_IN"),
                obel_cmt.wire(&format!("CCIO_TOP{i}")),
            );
        }
    }
    vrf.claim_node(&[bel.fwire("CLK_IN_MUX_HROUTE")]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_HROUTE"),
        obel_vcc.wire("VCC"),
    );
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN_MUX_HROUTE"),
            obel_cmt.wire(&format!("HROUTE{i}_{hr_lr}")),
        );
    }
    vrf.claim_node(&[bel.fwire("CLK_IN_MUX_PLL_CKINT")]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_PLL_CKINT"),
        bel.wire("CLK_IN_CKINT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_PLL_CKINT"),
        obel_pll0.wire("CLKFBOUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_PLL_CKINT"),
        obel_pll1.wire("CLKFBOUT"),
    );
    vrf.claim_node(&[bel.fwire("CLK_IN_MUX_TEST")]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_TEST"),
        obel_mmcm.wire("TMUXOUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_TEST"),
        obel_pll0.wire("TMUXOUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_TEST"),
        obel_pll1.wire("TMUXOUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_TEST"),
        obel_gtb.wire("CLK_OUT"),
    );
}

fn verify_bufgctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![
        ("CLK_I0", SitePinDir::In),
        ("CLK_I1", SitePinDir::In),
        ("CLK_OUT", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "BUFGCTRL", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let idx = bels::BUFGCTRL
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();

    let obel0 = vrf.find_bel_sibling(bel, bels::BUFGCE[idx * 3]);
    let obel1 = vrf.find_bel_sibling(bel, bels::BUFGCE[idx * 3 + 1]);
    let obel_p = vrf.find_bel_sibling(bel, bels::BUFGCTRL[(idx + 7) % 8]);
    let obel_n = vrf.find_bel_sibling(bel, bels::BUFGCTRL[(idx + 1) % 8]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I0"), obel0.wire_far("CLK_IN"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I0"), obel_p.wire_far("CLK_OUT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I0"), obel_n.wire_far("CLK_OUT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I1"), obel1.wire_far("CLK_IN"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I1"), obel_p.wire_far("CLK_OUT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I1"), obel_n.wire_far("CLK_OUT"));
}

fn verify_bufgce_div(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("CLK_IN", SitePinDir::In), ("CLK_OUT", SitePinDir::Out)];
    vrf.verify_bel(bel, "BUFGCE_DIV", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let idx = bels::BUFGCE_DIV
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();

    let obel = vrf.find_bel_sibling(bel, bels::BUFGCE[idx * 6 + 5]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire_far("CLK_IN"));
}

fn verify_mmcm(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = endev.edev.chips[bel.die].col_side(bel.col) == DirH::W;
    let hr_lr = if is_l { 'L' } else { 'R' };
    let kind = match endev.edev.kind {
        ChipKind::Ultrascale => "MMCME3_ADV",
        ChipKind::UltrascalePlus => "MMCM",
    };
    let pins = vec![
        ("CLKIN1", SitePinDir::In),
        ("CLKIN2", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT0B", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT1B", SitePinDir::Out),
        ("CLKOUT2", SitePinDir::Out),
        ("CLKOUT2B", SitePinDir::Out),
        ("CLKOUT3", SitePinDir::Out),
        ("CLKOUT3B", SitePinDir::Out),
        ("CLKOUT4", SitePinDir::Out),
        ("CLKOUT5", SitePinDir::Out),
        ("CLKOUT6", SitePinDir::Out),
        ("CLKFBOUT", SitePinDir::Out),
        ("CLKFBOUTB", SitePinDir::Out),
        ("TMUXOUT", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let (is_xp, obel_cmt) = if let Some(obel) = vrf.find_bel_delta(bel, 0, 0, bels::CMT) {
        (false, obel)
    } else {
        (true, vrf.find_bel_sibling(bel, bels::CMTXP))
    };
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_CMT);

    for pin in [
        "CLKIN1_MUX_HDISTR",
        "CLKIN2_MUX_HDISTR",
        "CLKFBIN_MUX_HDISTR",
    ] {
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_cmt.wire(&format!("HDISTR{i}_L")),
            );
        }
    }
    for pin in ["CLKIN1_MUX_HROUTE", "CLKIN2_MUX_HROUTE"] {
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_cmt.wire(&format!("HROUTE{i}_{hr_lr}")),
            );
        }
    }
    for pin in [
        "CLKIN1_MUX_BUFCE_ROW_DLY",
        "CLKIN2_MUX_BUFCE_ROW_DLY",
        "CLKFBIN_MUX_BUFCE_ROW_DLY",
    ] {
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            let obel_bufce_row = vrf.find_bel_sibling(bel, bels::BUFCE_ROW_CMT[i]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_bufce_row.wire("CLK_OUT_OPT_DLY"),
            );
        }
    }

    vrf.claim_node(&[bel.fwire("CLKIN1_MUX_DUMMY0")]);
    vrf.claim_node(&[bel.fwire("CLKIN2_MUX_DUMMY0")]);
    vrf.claim_node(&[bel.fwire("CLKFBIN_MUX_DUMMY0")]);
    vrf.claim_node(&[bel.fwire("CLKFBIN_MUX_DUMMY1")]);

    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_MUX_HDISTR"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_MUX_HROUTE"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKIN1"),
        bel.wire("CLKIN1_MUX_BUFCE_ROW_DLY"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_MUX_DUMMY0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_MUX_HDISTR"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_MUX_HROUTE"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKIN2"),
        bel.wire("CLKIN2_MUX_BUFCE_ROW_DLY"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_MUX_DUMMY0"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKFBIN"),
        bel.wire("CLKFBIN_MUX_HDISTR"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKFBIN"),
        bel.wire("CLKFBIN_MUX_BUFCE_ROW_DLY"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKFBIN"),
        bel.wire("CLKFBIN_MUX_DUMMY0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKFBIN"),
        bel.wire("CLKFBIN_MUX_DUMMY1"),
    );
    for pin in ["CLKIN1", "CLKIN2", "CLKFBIN"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
        if !is_xp {
            for i in 0..4 {
                vrf.claim_pip(bel.crd(), bel.wire(pin), obel_cmt.wire(&format!("CCIO{i}")));
            }
            if endev.edev.kind == ChipKind::Ultrascale {
                for i in 0..8 {
                    let ii = [0, 6, 13, 19, 26, 32, 39, 45][i];
                    let obel = vrf.find_bel_sibling(bel, bels::BITSLICE[ii]);
                    vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("PHY2CLB_FIFO_WRCLK"));
                }
            } else {
                for i in 0..8 {
                    vrf.claim_pip(
                        bel.crd(),
                        bel.wire(pin),
                        obel_cmt.wire(&format!("FIFO_WRCLK{i}")),
                    );
                }
            }
        } else {
            for i in 0..4 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(pin),
                    obel_cmt.wire(&format!("CCIO_BOT{i}")),
                );
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(pin),
                    obel_cmt.wire(&format!("CCIO_MID{i}")),
                );
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(pin),
                    obel_cmt.wire(&format!("CCIO_TOP{i}")),
                );
            }
        }
    }
    for pin in [
        "CLKIN1_MUX_HDISTR",
        "CLKIN1_MUX_HROUTE",
        "CLKIN1_MUX_BUFCE_ROW_DLY",
        "CLKIN2_MUX_HDISTR",
        "CLKIN2_MUX_HROUTE",
        "CLKIN2_MUX_BUFCE_ROW_DLY",
        "CLKFBIN_MUX_HDISTR",
        "CLKFBIN_MUX_BUFCE_ROW_DLY",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
    }
}

fn verify_pll(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = endev.edev.chips[bel.die].col_side(bel.col) == DirH::W;
    let hr_lr = if is_l { 'L' } else { 'R' };
    let is_pllxp = matches!(bel.slot, bels::PLLXP0 | bels::PLLXP1);
    let kind = if is_pllxp {
        "PLLXP"
    } else {
        match endev.edev.kind {
            ChipKind::Ultrascale => "PLLE3_ADV",
            ChipKind::UltrascalePlus => "PLL",
        }
    };
    let mut pins = vec![
        ("CLKIN", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT0B", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT1B", SitePinDir::Out),
        ("CLKFBOUT", SitePinDir::Out),
        ("TMUXOUT", SitePinDir::Out),
        ("CLKOUTPHY_P", SitePinDir::Out),
    ];
    if is_pllxp {
        pins.extend([
            ("CLKOUTPHY_DMCEN", SitePinDir::In),
            ("RST_DMC", SitePinDir::In),
            ("LOCKED_DMC", SitePinDir::Out),
        ]);
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_cmt = vrf.find_bel_sibling(bel, if is_pllxp { bels::CMTXP } else { bels::CMT });
    let obel_mmcm = vrf.find_bel_sibling(bel, bels::MMCM);
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_CMT);
    let has_hbm = vrf.find_bel_delta(bel, 0, 0, bels::HBM_REF_CLK0).is_some();

    for pin in ["CLKIN_MUX_HDISTR", "CLKFBIN_MUX_HDISTR"] {
        if has_hbm && pin == "CLKFBIN_MUX_HDISTR" {
            continue;
        }
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_cmt.wire(&format!("HDISTR{i}_L")),
            );
        }
    }
    vrf.claim_node(&[bel.fwire("CLKIN_MUX_HROUTE")]);
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKIN_MUX_HROUTE"),
            obel_cmt.wire(&format!("HROUTE{i}_{hr_lr}")),
        );
    }
    for pin in ["CLKIN_MUX_BUFCE_ROW_DLY", "CLKFBIN_MUX_BUFCE_ROW_DLY"] {
        if has_hbm && pin == "CLKFBIN_MUX_BUFCE_ROW_DLY" {
            continue;
        }
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            let obel_bufce_row = vrf.find_bel_sibling(bel, bels::BUFCE_ROW_CMT[i]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_bufce_row.wire("CLK_OUT_OPT_DLY"),
            );
        }
    }
    vrf.claim_node(&[bel.fwire("CLKIN_MUX_MMCM")]);
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKIN_MUX_MMCM"),
            obel_mmcm.wire(&format!("CLKOUT{i}")),
        );
    }

    for pin in [
        "CLKIN_MUX_HDISTR",
        "CLKIN_MUX_HROUTE",
        "CLKIN_MUX_BUFCE_ROW_DLY",
        "CLKIN_MUX_MMCM",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), bel.wire(pin));
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel_vcc.wire("VCC"));
    if is_pllxp {
        for i in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLKIN"),
                obel_cmt.wire(&format!("CCIO_BOT{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLKIN"),
                obel_cmt.wire(&format!("CCIO_MID{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLKIN"),
                obel_cmt.wire(&format!("CCIO_TOP{i}")),
            );
        }
    } else {
        for i in 0..4 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLKIN"),
                obel_cmt.wire(&format!("CCIO{i}")),
            );
        }
        if endev.edev.kind == ChipKind::Ultrascale {
            for i in 0..8 {
                let ii = [0, 6, 13, 19, 26, 32, 39, 45][i];
                let obel = vrf.find_bel_sibling(bel, bels::BITSLICE[ii]);
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLKIN"),
                    obel.wire("PHY2CLB_FIFO_WRCLK"),
                );
            }
        } else {
            for i in 0..8 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLKIN"),
                    obel_cmt.wire(&format!("FIFO_WRCLK{i}")),
                );
            }
        }
    }

    for pin in [
        "CLKIN_MUX_HDISTR",
        "CLKIN_MUX_HROUTE",
        "CLKIN_MUX_BUFCE_ROW_DLY",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
    }
    if has_hbm {
        vrf.claim_node(&[bel.fwire_far("CLKFBIN")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire_far("CLKFBIN"));
    } else {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKFBIN"),
            bel.wire("CLKFBIN_MUX_HDISTR"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKFBIN"),
            bel.wire("CLKFBIN_MUX_BUFCE_ROW_DLY"),
        );
        for pin in ["CLKFBIN_MUX_HDISTR", "CLKFBIN_MUX_BUFCE_ROW_DLY"] {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
        }
    }

    if is_pllxp {
        let obel_lpddrmc = vrf.find_bel_sibling(bel, bels::LPDDRMC);
        for pin in ["CLKOUTPHY_DMCEN", "RST_DMC"] {
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
        }
        let idx = if bel.slot == bels::PLLXP1 { 1 } else { 0 };
        vrf.verify_node(&[
            bel.fwire_far("CLKOUTPHY_DMCEN"),
            obel_lpddrmc.fwire(&format!("DMC_XPLL{idx}_CLKOUTPHY_EN")),
        ]);
        vrf.verify_node(&[
            bel.fwire_far("RST_DMC"),
            obel_lpddrmc.fwire(&format!("DMC_XPLL{idx}_RESET")),
        ]);
    }
}

fn verify_hbm_ref_clk(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("REF_CLK", SitePinDir::In)];
    vrf.verify_bel(bel, "HBM_REF_CLK", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_cmt = vrf.find_bel_sibling(bel, bels::CMT);
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_CMT);

    vrf.claim_node(&[bel.fwire("REF_CLK_MUX_HDISTR")]);
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REF_CLK_MUX_HDISTR"),
            obel_cmt.wire(&format!("HDISTR{i}_L")),
        );
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("REF_CLK_MUX_HDISTR"),
        obel_vcc.wire("VCC"),
    );
    vrf.claim_node(&[bel.fwire("REF_CLK_MUX_BUFCE_ROW_DLY")]);
    for i in 0..24 {
        let obel_bufce_row = vrf.find_bel_sibling(bel, bels::BUFCE_ROW_CMT[i]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REF_CLK_MUX_BUFCE_ROW_DLY"),
            obel_bufce_row.wire("CLK_OUT_OPT_DLY"),
        );
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("REF_CLK_MUX_BUFCE_ROW_DLY"),
        obel_vcc.wire("VCC"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("REF_CLK"),
        bel.wire("REF_CLK_MUX_HDISTR"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("REF_CLK"),
        bel.wire("REF_CLK_MUX_BUFCE_ROW_DLY"),
    );
}

fn verify_cmt(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let iocol = chip
        .cols_io
        .iter()
        .find(|iocol| iocol.col == bel.col)
        .unwrap();
    let is_hdio = iocol.regs[chip.row_to_reg(bel.row)] == IoRowKind::HdioL;
    let is_xp = bel.slot == bels::CMTXP;

    let is_l = endev.edev.chips[bel.die].col_side(bel.col) == DirH::W;
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_CMT);

    let obel_s = vrf.find_bel_delta(bel, 0, -(Chip::ROWS_PER_REG as isize), bel.slot);

    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("VDISTR{i}_T"))]);
        if let Some(ref obel_s) = obel_s {
            vrf.verify_node(&[
                bel.fwire(&format!("VDISTR{i}_B")),
                obel_s.fwire(&format!("VDISTR{i}_T")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("VDISTR{i}_B"))]);
        }
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_B")),
            bel.wire(&format!("VDISTR{i}_B_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_B")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_T")),
            bel.wire(&format!("VDISTR{i}_T_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_T")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_node(&[bel.fwire(&format!("VDISTR{i}_B_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_B_MUX")),
            bel.wire(&format!("VDISTR{i}_T")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_B_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
        vrf.claim_node(&[bel.fwire(&format!("VDISTR{i}_T_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_T_MUX")),
            bel.wire(&format!("VDISTR{i}_B")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_T_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
    }

    let obel_hr = find_hroute_src(endev, vrf, bel.cell);
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        if is_l {
            vrf.verify_node(&[
                bel.fwire(&format!("HROUTE{i}_R")),
                obel_hr.fwire(&format!("HROUTE{i}_L")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R"))]);
        }
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
    }

    if endev.edev.kind == ChipKind::Ultrascale {
        let obel_hd = find_hdistr_src(endev, vrf, bel.cell);
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_R")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
        }
    } else {
        let obel_hd = find_hdistr_src(endev, vrf, bel.cell);
        for i in 0..24 {
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_L")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
            if bel.slot == bels::CMTXP {
                vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
            }
        }
        // R is a lie and goes nowhere.
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_R"))]);
        }
    }
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            bel.wire(&format!("HDISTR{i}_L_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            bel.wire(&format!("HDISTR{i}_R_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L_MUX")),
            bel.wire(&format!("HDISTR{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L_MUX")),
            bel.wire(&format!("HDISTR{i}_OUT_MUX")),
        );
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_R_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R_MUX")),
            bel.wire(&format!("HDISTR{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R_MUX")),
            bel.wire(&format!("HDISTR{i}_OUT_MUX")),
        );
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_OUT_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_OUT_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
        let obel = vrf.find_bel_sibling(bel, bels::BUFCE_ROW_CMT[i]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_OUT_MUX")),
            obel.wire("CLK_OUT"),
        );
    }

    for i in 0..24 {
        let pin = format!("OUT_MUX{i}");
        vrf.claim_node(&[bel.fwire(&pin)]);
        vrf.claim_pip(bel.crd(), bel.wire(&pin), obel_vcc.wire("VCC"));
        for j in 0..3 {
            vrf.claim_node(&[bel.fwire(&format!("OUT_MUX{i}_DUMMY{j}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&pin),
                bel.wire(&format!("OUT_MUX{i}_DUMMY{j}")),
            );
        }
        let obel = vrf.find_bel_sibling(bel, bels::BUFGCE[i]);
        vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire("CLK_OUT"));
        for j in 0..8 {
            let obel = vrf.find_bel_sibling(bel, bels::BUFGCTRL[j]);
            vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire("CLK_OUT"));
        }
        for j in 0..4 {
            let obel = vrf.find_bel_sibling(bel, bels::BUFGCE_DIV[j]);
            vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire("CLK_OUT"));
        }
    }

    for i in 0..24 {
        let obel = vrf.find_bel_sibling(bel, bels::GCLK_TEST_BUF_CMT[i]);
        vrf.claim_node(&[obel.fwire("CLK_IN")]);
        vrf.claim_pip(bel.crd(), obel.wire("CLK_IN"), obel_vcc.wire("VCC"));
        for j in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                obel.wire("CLK_IN"),
                bel.wire(&format!("HDISTR{j}_L")),
            );
        }
    }

    if !is_xp {
        for (i, dy, hpio_slot, hrio_slot, hdiol_slot) in [
            (0, 0, bels::HPIOB0, bels::HRIOB0, bels::HDIOB0),
            (1, 0, bels::HPIOB2, bels::HRIOB2, bels::HDIOB2),
            (2, -30, bels::HPIOB21, bels::HRIOB21, bels::HDIOB12),
            (3, -30, bels::HPIOB23, bels::HRIOB23, bels::HDIOB10),
        ] {
            if let Some(obel) = vrf
                .find_bel_delta(bel, 0, dy, hpio_slot)
                .or_else(|| vrf.find_bel_delta(bel, 0, dy, hrio_slot))
            {
                let obel_slot = endev.edev.egrid.db.bel_slots.key(obel.slot);
                vrf.verify_node(&[
                    bel.fwire(&format!("CCIO{i}")),
                    obel.fwire(if obel_slot.starts_with("HRIO") {
                        "DOUT"
                    } else {
                        "I"
                    }),
                ]);
            } else {
                if chip.config_kind == ConfigKind::CsecV2 {
                    let obel = vrf.find_bel_delta(bel, 0, dy, hdiol_slot).unwrap();
                    vrf.verify_node(&[bel.fwire(&format!("CCIO{i}")), obel.fwire("I")]);
                } else {
                    vrf.claim_node(&[bel.fwire(&format!("CCIO{i}"))]);
                }
            }
        }

        if endev.edev.kind == ChipKind::UltrascalePlus {
            if is_hdio {
                for i in 0..8 {
                    vrf.claim_node(&[bel.fwire(&format!("FIFO_WRCLK{i}"))]);
                }
            } else {
                for i in 0..8 {
                    let ii = match i % 2 {
                        0 => 0,
                        1 => 6,
                        _ => unreachable!(),
                    };
                    let obel = vrf
                        .find_bel_delta(bel, 0, -30 + 15 * (i / 2), bels::BITSLICE[ii])
                        .unwrap();
                    vrf.verify_node(&[
                        bel.fwire(&format!("FIFO_WRCLK{i}")),
                        obel.fwire("PHY2CLB_FIFO_WRCLK"),
                    ]);
                }
            }
        }
    } else {
        let obel_lpddrmc = vrf.find_bel_sibling(bel, bels::LPDDRMC);
        for i in 0..4 {
            vrf.verify_node(&[
                bel.fwire(&format!("CCIO_BOT{i}")),
                obel_lpddrmc.fwire(&format!(
                    "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK{ii}",
                    ii = 1 + i
                )),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("CCIO_TOP{i}")),
                obel_lpddrmc.fwire(&format!(
                    "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK{ii}",
                    ii = 5 + i
                )),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("CCIO_MID{i}")),
                obel_lpddrmc.fwire(&format!("IF_XPIO_MMCM_DMC_OABUT_XPIO_CCIO{i}",)),
            ]);
        }
    }

    if endev.edev.kind == ChipKind::Ultrascale {
        for i in 0..6 {
            for bt in ['B', 'T'] {
                let pin = format!("XIPHY_CLK{i}_{bt}");
                vrf.claim_node(&[bel.fwire(&pin)]);
                vrf.claim_pip(bel.crd(), bel.wire(&pin), obel_vcc.wire("VCC"));
                for j in 0..24 {
                    vrf.claim_pip(bel.crd(), bel.wire(&pin), bel.wire(&format!("HDISTR{j}_L")));
                }
            }
        }
    }
}

fn verify_bitslice_rx_tx(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::BITSLICE
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let bidx = idx / 13;
    let bsidx = idx % 13;
    let nidx = usize::from(bsidx >= 6);
    let sidx = bsidx - nidx * 6;
    let obel_bsctl = vrf.find_bel_sibling(bel, bels::BITSLICE_CONTROL[bidx * 2 + nidx]);
    let obel_feed = vrf.find_bel_sibling(bel, bels::XIPHY_FEEDTHROUGH[bidx]);
    let obel_bstx = vrf.find_bel_sibling(bel, bels::BITSLICE_T[bidx * 2 + nidx]);
    let mut pins = vec![
        // mux
        ("RX_CLK", SitePinDir::In),
        ("RX_CLK_C", SitePinDir::In),
        ("RX_CLK_C_B", SitePinDir::In),
        ("TX_OCLK", SitePinDir::In),
        ("RX_CLKDIV", SitePinDir::In),  // alias of RX_CLK
        ("TX_CLK", SitePinDir::In),     // alias of RX_CLK
        ("TX_OCLKDIV", SitePinDir::In), // alias of RX_CLK
        // to IOB
        ("TX_Q", SitePinDir::Out),
        ("RX_D", SitePinDir::In),
        // to BSCTL
        ("BS2CTL_TX_DDR_PHASE_SEL", SitePinDir::Out),
        ("TX_VTC_READY", SitePinDir::Out),
        ("RX_VTC_READY", SitePinDir::Out),
        ("BS2CTL_IDELAY_DELAY_FORMAT", SitePinDir::Out),
        ("BS2CTL_ODELAY_DELAY_FORMAT", SitePinDir::Out),
        ("RX_DQS_OUT", SitePinDir::Out),
        ("BS2CTL_RX_DDR_EN_DQS", SitePinDir::Out),
        ("BS2CTL_RX_P0_DQ_OUT", SitePinDir::Out),
        ("BS2CTL_RX_N0_DQ_OUT", SitePinDir::Out),
        // to RCLK
        ("PHY2CLB_FIFO_WRCLK", SitePinDir::Out),
        // cascade stuff
        ("RX2TX_CASC_RETURN_IN", SitePinDir::In),
        ("TX2RX_CASC_IN", SitePinDir::In),
        ("TX2RX_CASC_OUT", SitePinDir::Out),
    ];

    for (pin, opin) in [
        ("TX_CTRL_CLK", format!("ODELAY_CTRL_CLK{sidx}")),
        ("TX_CTRL_CE", format!("ODELAY_CE_OUT{sidx}")),
        ("TX_CTRL_INC", format!("ODELAY_INC_OUT{sidx}")),
        ("TX_CTRL_LD", format!("ODELAY_LD_OUT{sidx}")),
        ("TX_DIV2_CLK", format!("DIV2_CLK_OUT{sidx}")),
        ("TX_DIV4_CLK", format!("DIV_CLK_OUT{sidx}")),
        ("TX_DDR_CLK", format!("DDR_CLK_OUT{sidx}")),
        ("CTL2BS_DYNAMIC_MODE_EN", format!("DYNAMIC_MODE_EN{sidx}")),
        ("CTL2BS_TX_DDR_PHASE_SEL", format!("TX_DATA_PHASE{sidx}")),
        ("TX_TOGGLE_DIV2_SEL", format!("TOGGLE_DIV2_SEL{sidx}")),
        ("TX_MUX_360_P_SEL", format!("PH02_DIV2_360_{sidx}")),
        ("TX_MUX_360_N_SEL", format!("PH13_DIV2_360_{sidx}")),
        ("TX_MUX_720_P0_SEL", format!("PH0_DIV_720_{sidx}")),
        ("TX_MUX_720_P1_SEL", format!("PH1_DIV_720_{sidx}")),
        ("TX_MUX_720_P2_SEL", format!("PH2_DIV_720_{sidx}")),
        ("TX_MUX_720_P3_SEL", format!("PH3_DIV_720_{sidx}")),
        ("TX_WL_TRAIN", format!("WL_TRAIN{sidx}")),
        ("TX_BS_RESET", format!("TX_BS_RESET{sidx}")),
        ("RX_CLK_P", format!("PDQS_OUT{sidx}")),
        ("RX_CLK_N", format!("NDQS_OUT{sidx}")),
        ("RX_CTRL_CLK", format!("IDELAY_CTRL_CLK{sidx}")),
        ("RX_CTRL_CE", format!("IDELAY_CE_OUT{sidx}")),
        ("RX_CTRL_INC", format!("IDELAY_INC_OUT{sidx}")),
        ("RX_CTRL_LD", format!("IDELAY_LD_OUT{sidx}")),
        ("RX_DCC0", format!("RX_DCC{sidx:02}_0")),
        ("RX_DCC1", format!("RX_DCC{sidx:02}_1")),
        ("RX_DCC2", format!("RX_DCC{sidx:02}_2")),
        ("RX_DCC3", format!("RX_DCC{sidx:02}_3")),
        ("RX_BS_RESET", format!("RX_BS_RESET{sidx}")),
        ("CTL2BS_FIFO_BYPASS", format!("IFIFO_BYPASS{sidx}")),
    ] {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_bsctl.wire(&opin));
    }

    let ul = if nidx == 1 { "UPP" } else { "LOW" };
    let mut pins_feed = vec![
        (
            "CTL2BS_RX_RECALIBRATE_EN",
            format!("CTL2BS_REFCLK_EN_{ul}_SMX{sidx}"),
        ),
        ("CLB2PHY_FIFO_CLK", format!("CLB2PHY_FIFO_CLK_SMX{bsidx}")),
    ];

    if endev.edev.kind == ChipKind::Ultrascale {
        pins_feed.extend([
            ("RX_RESET_B", format!("CLB2PHY_IDELAY_RST_B_SMX{bsidx}")),
            ("TX_REGRST_B", format!("CLB2PHY_ODELAY_RST_B_SMX{bsidx}")),
            ("RX_RST_B", format!("CLB2PHY_RXBIT_RST_B_SMX{bsidx}")),
            ("TX_RST_B", format!("CLB2PHY_TXBIT_RST_B_SMX{bsidx}")),
        ]);
    } else {
        pins_feed.extend([
            ("RX_RESET", format!("CLB2PHY_IDELAY_RST_SMX{bsidx}")),
            ("TX_REGRST", format!("CLB2PHY_ODELAY_RST_SMX{bsidx}")),
            ("RX_RST", format!("CLB2PHY_RXBIT_RST_SMX{bsidx}")),
            ("TX_RST", format!("CLB2PHY_TXBIT_RST_SMX{bsidx}")),
        ]);
    }

    for (pin, opin) in pins_feed {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_feed.wire(&opin));
    }

    let rx_ctrl_dly: [_; 9] = core::array::from_fn(|i| format!("RX_CTRL_DLY{i}"));
    let tx_ctrl_dly: [_; 9] = core::array::from_fn(|i| format!("TX_CTRL_DLY{i}"));
    for (i, pin) in rx_ctrl_dly.iter().enumerate() {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_bsctl.wire(&format!("IDELAY{sidx:02}_OUT{i}")),
        );
    }
    for (i, pin) in tx_ctrl_dly.iter().enumerate() {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_bsctl.wire(&format!("ODELAY{sidx:02}_OUT{i}")),
        );
    }
    // to BSCTL
    let rx_cntvalueout: [_; 9] = core::array::from_fn(|i| format!("BS2CTL_RX_CNTVALUEOUT{i}"));
    for pin in &rx_cntvalueout {
        pins.push((pin, SitePinDir::Out));
    }
    let tx_cntvalueout: [_; 9] = core::array::from_fn(|i| format!("BS2CTL_TX_CNTVALUEOUT{i}"));
    for pin in &tx_cntvalueout {
        pins.push((pin, SitePinDir::Out));
    }
    let idelay_fixed_dly_ratio: [_; 18] =
        core::array::from_fn(|i| format!("BS2CTL_IDELAY_FIXED_DLY_RATIO{i}"));
    for pin in &idelay_fixed_dly_ratio {
        pins.push((pin, SitePinDir::Out));
    }
    let odelay_fixed_dly_ratio: [_; 18] =
        core::array::from_fn(|i| format!("BS2CTL_ODELAY_FIXED_DLY_RATIO{i}"));
    for pin in &odelay_fixed_dly_ratio {
        pins.push((pin, SitePinDir::Out));
    }

    pins.push(("TX_TBYTE_IN", SitePinDir::In));
    vrf.claim_pip(bel.crd(), bel.wire("TX_TBYTE_IN"), obel_bstx.wire("Q"));

    vrf.verify_bel(bel, "BITSLICE_RX_TX", &pins, &["DYN_DCI_OUT_INT"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    vrf.claim_node(&[bel.fwire_far("RX_CLK")]);

    if endev.edev.kind == ChipKind::Ultrascale {
        let obel = vrf.find_bel_sibling(bel, bels::CMT);
        let bt = if bidx < 2 { 'B' } else { 'T' };
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("RX_CLK"),
                obel.wire(&format!("XIPHY_CLK{i}_{bt}")),
            );
            for pin in ["RX_CLK_C", "RX_CLK_C_B", "TX_OCLK"] {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(pin),
                    obel.wire(&format!("XIPHY_CLK{i}_{bt}")),
                );
            }
        }
    } else {
        let obel = vrf.find_bel_sibling(bel, bels::XIPHY_BYTE);
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("RX_CLK"),
                obel.wire(&format!("XIPHY_CLK{i}")),
            );
            for pin in ["RX_CLK_C", "RX_CLK_C_B", "TX_OCLK"] {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(pin),
                    obel.wire(&format!("XIPHY_CLK{i}")),
                );
            }
        }
    };
    for pin in ["RX_CLK", "RX_CLKDIV", "TX_CLK", "TX_OCLKDIV"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far("RX_CLK"));
    }

    vrf.claim_pip(bel.crd(), bel.wire("RX_D"), bel.wire_far("RX_D"));

    if bsidx != 12 {
        let obel_bs = vrf.find_bel_sibling(bel, bels::BITSLICE[idx + 1]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("TX2RX_CASC_IN"),
            obel_bs.wire("TX2RX_CASC_OUT"),
        );
    }
    if bsidx != 0 {
        let obel_bs = vrf.find_bel_sibling(bel, bels::BITSLICE[idx - 1]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("RX2TX_CASC_RETURN_IN"),
            obel_bs.wire("RX_Q5"),
        );
    }

    vrf.claim_pip(
        bel.crd(),
        bel.wire("DYN_DCI_OUT"),
        bel.wire("DYN_DCI_OUT_INT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("DYN_DCI_OUT"),
        obel_bsctl.wire(&format!("DYN_DCI_OUT{sidx}")),
    );
}

fn verify_bitslice_tx(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::BITSLICE_T
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let obel_bsctl = vrf.find_bel_sibling(bel, bels::BITSLICE_CONTROL[idx]);
    let obel_feed = vrf.find_bel_sibling(bel, bels::XIPHY_FEEDTHROUGH[idx / 2]);
    let mut pins = vec![
        // mux
        ("CLK", SitePinDir::In),
        // to BITSLICE_RX_TX.TBYTE_IN [all of them]
        ("Q", SitePinDir::Out),
        // to BSCTL
        ("BS2CTL_TX_DDR_PHASE_SEL", SitePinDir::Out),
        ("VTC_READY", SitePinDir::Out),
        // dummy stuff?
        ("CDATAIN0", SitePinDir::In),
        ("CDATAIN1", SitePinDir::In),
        ("CDATAOUT", SitePinDir::Out),
    ];

    for (pin, opin) in [
        ("CTRL_CE", "TRISTATE_ODELAY_CE_OUT"),
        ("CTRL_INC", "TRISTATE_ODELAY_INC_OUT"),
        ("CTRL_LD", "TRISTATE_ODELAY_LD_OUT"),
        ("CTRL_CLK", "ODELAY_CTRL_CLK7"),
        ("DIV2_CLK", "DIV2_CLK_OUT7"),
        ("DIV4_CLK", "DIV_CLK_OUT7"),
        ("DDR_CLK", "DDR_CLK_OUT7"),
        ("CTL2BS_DYNAMIC_MODE_EN", "DYNAMIC_MODE_EN7"),
        ("CTL2BS_TX_DDR_PHASE_SEL", "TX_DATA_PHASE7"),
        ("FORCE_OE_B", "FORCE_OE_B"),
        ("TOGGLE_DIV2_SEL", "TOGGLE_DIV2_SEL7"),
        ("TX_MUX_360_P_SEL", "PH02_DIV2_360_7"),
        ("TX_MUX_360_N_SEL", "PH13_DIV2_360_7"),
        ("TX_MUX_720_P0_SEL", "PH0_DIV_720_7"),
        ("TX_MUX_720_P1_SEL", "PH1_DIV_720_7"),
        ("TX_MUX_720_P2_SEL", "PH2_DIV_720_7"),
        ("TX_MUX_720_P3_SEL", "PH3_DIV_720_7"),
        ("BS_RESET", "BS_RESET_TRI"),
    ] {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_bsctl.wire(opin));
    }
    let feed_pins = if endev.edev.kind == ChipKind::Ultrascale {
        [
            (
                "REGRST_B",
                "CLB2PHY_TRISTATE_ODELAY_RST_B_SMX0",
                "CLB2PHY_TRISTATE_ODELAY_RST_B_SMX1",
            ),
            (
                "RST_B",
                "CLB2PHY_TXBIT_TRI_RST_B_SMX0",
                "CLB2PHY_TXBIT_TRI_RST_B_SMX1",
            ),
        ]
    } else {
        [
            (
                "REGRST",
                "CLB2PHY_TRISTATE_ODELAY_RST_SMX0",
                "CLB2PHY_TRISTATE_ODELAY_RST_SMX1",
            ),
            (
                "RST",
                "CLB2PHY_TXBIT_TRI_RST_SMX0",
                "CLB2PHY_TXBIT_TRI_RST_SMX1",
            ),
        ]
    };
    for (pin, opin_l, opin_u) in feed_pins {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_feed.wire(if idx.is_multiple_of(2) {
                opin_l
            } else {
                opin_u
            }),
        );
    }
    let ctrl_dly: [_; 9] = core::array::from_fn(|i| format!("CTRL_DLY{i}"));
    for (i, pin) in ctrl_dly.iter().enumerate() {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_bsctl.wire(&format!("TRISTATE_ODELAY_OUT{i}")),
        );
    }
    let d: [_; 8] = core::array::from_fn(|i| format!("D{i}"));
    for (i, pin) in d.iter().enumerate() {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_bsctl.wire(&format!("EN_DIV_DLY_OE{i}")),
        );
    }
    // to BSCTL
    let cntvalueout: [_; 9] = core::array::from_fn(|i| format!("BS2CTL_CNTVALUEOUT{i}"));
    for pin in &cntvalueout {
        pins.push((pin, SitePinDir::Out));
    }

    vrf.verify_bel(bel, "BITSLICE_TX", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    if endev.edev.kind == ChipKind::Ultrascale {
        let obel = vrf.find_bel_sibling(bel, bels::CMT);
        let bt = if idx < 4 { 'B' } else { 'T' };
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLK"),
                obel.wire(&format!("XIPHY_CLK{i}_{bt}")),
            );
        }
    } else {
        let obel = vrf.find_bel_sibling(bel, bels::XIPHY_BYTE);
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLK"),
                obel.wire(&format!("XIPHY_CLK{i}")),
            );
        }
    };
}

fn verify_bitslice_control(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let idx = bels::BITSLICE_CONTROL
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let obel_pll_sel = vrf.find_bel_sibling(bel, bels::PLL_SELECT[idx]);
    let obel_bstx = vrf.find_bel_sibling(bel, bels::BITSLICE_T[idx]);
    let obel_feed = vrf.find_bel_sibling(bel, bels::XIPHY_FEEDTHROUGH[idx / 2]);

    let mut opins = vec![];
    // to PLL_SELECT
    for pin in ["REFCLK_DFD", "PLL_CLK_EN"] {
        opins.push(pin.to_string());
    }
    // to RIU
    opins.push("RIU2CLB_VALID".to_string());
    for i in 0..16 {
        opins.push(format!("RIU2CLB_RD_DATA{i}"));
    }
    // to BITSLICE_TX
    for pin in [
        "TRISTATE_ODELAY_CE_OUT",
        "TRISTATE_ODELAY_INC_OUT",
        "TRISTATE_ODELAY_LD_OUT",
        "FORCE_OE_B",
        "BS_RESET_TRI",
    ] {
        opins.push(pin.to_string());
    }
    for i in 0..8 {
        opins.push(format!("EN_DIV_DLY_OE{i}"));
    }
    for i in 0..9 {
        opins.push(format!("TRISTATE_ODELAY_OUT{i}"));
    }
    // to BITSLICE_TX and BITSLICE_RX_TX
    for i in 0..8 {
        for p in [
            format!("ODELAY_CTRL_CLK{i}"),
            format!("DYNAMIC_MODE_EN{i}"),
            format!("TOGGLE_DIV2_SEL{i}"),
            format!("TX_DATA_PHASE{i}"),
            format!("DIV2_CLK_OUT{i}"),
            format!("DIV_CLK_OUT{i}"),
            format!("DDR_CLK_OUT{i}"),
            format!("PH02_DIV2_360_{i}"),
            format!("PH13_DIV2_360_{i}"),
            format!("PH0_DIV_720_{i}"),
            format!("PH1_DIV_720_{i}"),
            format!("PH2_DIV_720_{i}"),
            format!("PH3_DIV_720_{i}"),
        ] {
            opins.push(p);
        }
    }
    // to BITSLICE_RX_TX
    for i in 0..7 {
        for p in [
            format!("IDELAY_CTRL_CLK{i}"),
            format!("IDELAY_CE_OUT{i}"),
            format!("IDELAY_INC_OUT{i}"),
            format!("IDELAY_LD_OUT{i}"),
            format!("ODELAY_CE_OUT{i}"),
            format!("ODELAY_INC_OUT{i}"),
            format!("ODELAY_LD_OUT{i}"),
            format!("WL_TRAIN{i}"),
            format!("RX_BS_RESET{i}"),
            format!("TX_BS_RESET{i}"),
            format!("PDQS_OUT{i}"),
            format!("NDQS_OUT{i}"),
            format!("RX_DCC{i:02}_0"),
            format!("RX_DCC{i:02}_1"),
            format!("RX_DCC{i:02}_2"),
            format!("RX_DCC{i:02}_3"),
            format!("IFIFO_BYPASS{i}"),
        ] {
            opins.push(p);
        }
        for j in 0..9 {
            for p in [
                format!("IDELAY{i:02}_OUT{j}"),
                format!("ODELAY{i:02}_OUT{j}"),
            ] {
                opins.push(p);
            }
        }
    }
    // to IOB, via a mux associated with BITSLICE_RX_TX
    for i in 0..7 {
        opins.push(format!("DYN_DCI_OUT{i}"));
    }
    // to XIPHY_FEEDTHROUGH
    for i in 0..7 {
        opins.push(format!("REFCLK_EN{i}"));
    }
    for pin in [
        // to other BSCTL
        "CLK_TO_EXT_SOUTH",
        "CLK_TO_EXT_NORTH",
        "PDQS_GT_OUT",
        "NDQS_GT_OUT",
        // to XIPHY_FEEDTHROUGH
        "LOCAL_DIV_CLK",
    ] {
        opins.push(pin.to_string());
    }

    let mut ipins = vec![];
    // from PLL_SELECT
    ipins.push("PLL_CLK".to_string());
    vrf.claim_pip(bel.crd(), bel.wire("PLL_CLK"), obel_pll_sel.wire("Z"));
    // from BITSLICE_TX
    ipins.push("BS2CTL_RIU_TX_DATA_PHASE7".to_string());
    vrf.claim_pip(
        bel.crd(),
        bel.wire("BS2CTL_RIU_TX_DATA_PHASE7"),
        obel_bstx.wire("BS2CTL_TX_DDR_PHASE_SEL"),
    );
    ipins.push("TRISTATE_VTC_READY".to_string());
    vrf.claim_pip(
        bel.crd(),
        bel.wire("TRISTATE_VTC_READY"),
        obel_bstx.wire("VTC_READY"),
    );
    for i in 0..9 {
        let pin = format!("TRISTATE_ODELAY_IN{i}");
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&pin),
            obel_bstx.wire(&format!("BS2CTL_CNTVALUEOUT{i}")),
        );
        ipins.push(pin);
    }

    // from BITSLICE_RX_TX
    for i in 0..7 {
        let obel = if idx.is_multiple_of(2) && i == 6 {
            None
        } else {
            let ii = idx / 2 * 13 + idx % 2 * 6 + i;
            Some(vrf.find_bel_sibling(bel, bels::BITSLICE[ii]))
        };
        for (pin, opin) in [
            (
                format!("BS2CTL_RIU_TX_DATA_PHASE{i}"),
                "BS2CTL_TX_DDR_PHASE_SEL",
            ),
            (format!("RX_PDQ{i}_IN"), "BS2CTL_RX_P0_DQ_OUT"),
            (format!("RX_NDQ{i}_IN"), "BS2CTL_RX_N0_DQ_OUT"),
            (format!("FIXED_IDELAY{i:02}"), "BS2CTL_IDELAY_DELAY_FORMAT"),
            (format!("FIXED_ODELAY{i:02}"), "BS2CTL_ODELAY_DELAY_FORMAT"),
            (format!("VTC_READY_IDELAY{i:02}"), "RX_VTC_READY"),
            (format!("VTC_READY_ODELAY{i:02}"), "TX_VTC_READY"),
            (format!("DQS_IN{i}"), "RX_DQS_OUT"),
            (format!("BS2CTL_RIU_BS_DQS_EN{i}"), "BS2CTL_RX_DDR_EN_DQS"),
        ] {
            if let Some(ref obel) = obel {
                vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire(opin));
            }
            ipins.push(pin);
        }
        for j in 0..9 {
            for (pin, opin) in [
                (
                    format!("IDELAY{i:02}_IN{j}"),
                    format!("BS2CTL_RX_CNTVALUEOUT{j}"),
                ),
                (
                    format!("ODELAY{i:02}_IN{j}"),
                    format!("BS2CTL_TX_CNTVALUEOUT{j}"),
                ),
            ] {
                if let Some(ref obel) = obel {
                    vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire(&opin));
                }
                ipins.push(pin);
            }
        }
        for j in 0..18 {
            for (pin, opin) in [
                (
                    format!("FIXDLYRATIO_IDELAY{i:02}_{j}"),
                    format!("BS2CTL_IDELAY_FIXED_DLY_RATIO{j}"),
                ),
                (
                    format!("FIXDLYRATIO_ODELAY{i:02}_{j}"),
                    format!("BS2CTL_ODELAY_FIXED_DLY_RATIO{j}"),
                ),
            ] {
                if let Some(ref obel) = obel {
                    vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire(&opin));
                }
                ipins.push(pin);
            }
        }
    }

    for (pin, opin) in [
        (
            "CLK_STOP",
            match idx % 2 {
                0 => "XIPHY_CLK_STOP_CTRL_LOW",
                1 => "XIPHY_CLK_STOP_CTRL_UPP",
                _ => unreachable!(),
            },
        ),
        (
            "SCAN_INT",
            match idx % 2 {
                0 => "SCAN_INT_LOWER",
                1 => "SCAN_INT_UPPER",
                _ => unreachable!(),
            },
        ),
        if endev.edev.kind == ChipKind::Ultrascale {
            (
                "CLB2PHY_CTRL_RST_B",
                match idx % 2 {
                    0 => "CLB2PHY_CTRL_RST_B_LOW_SMX",
                    1 => "CLB2PHY_CTRL_RST_B_UPP_SMX",
                    _ => unreachable!(),
                },
            )
        } else {
            (
                "CLB2PHY_CTRL_RST",
                match idx % 2 {
                    0 => "CLB2PHY_CTRL_RST_LOW_SMX",
                    1 => "CLB2PHY_CTRL_RST_UPP_SMX",
                    _ => unreachable!(),
                },
            )
        },
    ] {
        ipins.push(pin.to_string());
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_feed.wire(opin));
    }

    for pin in [
        // from other BSCTL in byte
        "PDQS_GT_IN",
        "NDQS_GT_IN",
        // from BSCTL in another byte
        "CLK_FROM_EXT",
    ] {
        ipins.push(pin.to_string());
    }
    let obel_bsctl_on = vrf.find_bel_sibling(bel, bels::BITSLICE_CONTROL[idx ^ 1]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("PDQS_GT_IN"),
        obel_bsctl_on.wire("PDQS_GT_OUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NDQS_GT_IN"),
        obel_bsctl_on.wire("NDQS_GT_OUT"),
    );
    if endev.edev.kind == ChipKind::Ultrascale {
        let is_from_n = idx < 4;
        let obel_bsctl_ob = vrf.find_bel_sibling(
            bel,
            bels::BITSLICE_CONTROL[if is_from_n { idx + 2 } else { idx - 2 }],
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_FROM_EXT"),
            obel_bsctl_ob.wire(if is_from_n {
                "CLK_TO_EXT_SOUTH"
            } else {
                "CLK_TO_EXT_NORTH"
            }),
        );
    } else {
        let is_from_n = bel.row < chip.row_rclk(bel.row);
        let obel_bsctl_ob = vrf
            .find_bel_delta(bel, 0, if is_from_n { 15 } else { -15 }, bel.slot)
            .unwrap();
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_FROM_EXT"),
            bel.wire_far("CLK_FROM_EXT"),
        );
        vrf.verify_node(&[
            bel.fwire_far("CLK_FROM_EXT"),
            obel_bsctl_ob.fwire(if is_from_n {
                "CLK_TO_EXT_SOUTH"
            } else {
                "CLK_TO_EXT_NORTH"
            }),
        ]);
    }

    let mut pins = vec![];
    for pin in &opins {
        pins.push((&pin[..], SitePinDir::Out));
    }
    for pin in &ipins {
        pins.push((&pin[..], SitePinDir::In));
    }
    vrf.verify_bel(bel, "BITSLICE_CONTROL", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_pll_select(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let idx = bels::PLL_SELECT
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let pins = vec![
        ("D0", SitePinDir::In),
        ("D1", SitePinDir::In),
        ("REFCLK_DFD", SitePinDir::In),
        ("PLL_CLK_EN", SitePinDir::In),
        ("Z", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "PLL_SELECT_SITE", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel_pll0 = vrf.get_bel(bel.cell.with_row(chip.row_rclk(bel.row)).bel(bels::PLL0));
    let obel_pll1 = vrf.get_bel(bel.cell.with_row(chip.row_rclk(bel.row)).bel(bels::PLL1));
    vrf.claim_pip(bel.crd(), bel.wire("D0"), bel.wire_far("D0"));
    vrf.claim_pip(bel.crd(), bel.wire("D1"), bel.wire_far("D1"));
    vrf.verify_node(&[bel.fwire_far("D0"), obel_pll0.fwire("CLKOUTPHY_P")]);
    vrf.verify_node(&[bel.fwire_far("D1"), obel_pll1.fwire("CLKOUTPHY_P")]);

    let obel_bsctl = vrf.find_bel_sibling(bel, bels::BITSLICE_CONTROL[idx]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("REFCLK_DFD"),
        obel_bsctl.wire("REFCLK_DFD"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("PLL_CLK_EN"),
        obel_bsctl.wire("PLL_CLK_EN"),
    );
}

fn verify_riu_or(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::RIU_OR
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let obel_l = vrf.find_bel_sibling(bel, bels::BITSLICE_CONTROL[idx * 2]);
    let obel_u = vrf.find_bel_sibling(bel, bels::BITSLICE_CONTROL[idx * 2 + 1]);
    let mut ipins = vec![];
    for (obel, ul) in [(obel_l, "LOW"), (obel_u, "UPP")] {
        let pin = format!("RIU_RD_VALID_{ul}");
        vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire("RIU2CLB_VALID"));
        ipins.push(pin);
        for i in 0..16 {
            let pin = format!("RIU_RD_DATA_{ul}{i}");
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&pin),
                obel.wire(&format!("RIU2CLB_RD_DATA{i}")),
            );
            ipins.push(pin);
        }
    }

    let pins: Vec<_> = ipins.iter().map(|x| (&x[..], SitePinDir::In)).collect();
    vrf.verify_bel(bel, "RIU_OR", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_xiphy_feedthrough(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::XIPHY_FEEDTHROUGH
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let obel_bsctl_l = vrf.find_bel_sibling(bel, bels::BITSLICE_CONTROL[idx * 2]);
    let obel_bsctl_u = vrf.find_bel_sibling(bel, bels::BITSLICE_CONTROL[idx * 2 + 1]);
    let mut pins = vec![
        // to BSCTL
        ("SCAN_INT_LOWER", SitePinDir::Out),
        ("SCAN_INT_UPPER", SitePinDir::Out),
        ("XIPHY_CLK_STOP_CTRL_LOW", SitePinDir::Out),
        ("XIPHY_CLK_STOP_CTRL_UPP", SitePinDir::Out),
        // from BSCTL
        ("DIV_CLK_OUT_LOW", SitePinDir::In),
        ("DIV_CLK_OUT_UPP", SitePinDir::In),
        // dummy ins
        ("RCLK2PHY_CLKDR", SitePinDir::In),
        ("RCLK2PHY_SHIFTDR", SitePinDir::In),
    ];
    if endev.edev.kind == ChipKind::Ultrascale {
        pins.extend([
            // to BSCTL
            ("CLB2PHY_CTRL_RST_B_LOW_SMX", SitePinDir::Out),
            ("CLB2PHY_CTRL_RST_B_UPP_SMX", SitePinDir::Out),
            // to BITSLICE_TX
            ("CLB2PHY_TRISTATE_ODELAY_RST_B_SMX0", SitePinDir::Out),
            ("CLB2PHY_TRISTATE_ODELAY_RST_B_SMX1", SitePinDir::Out),
            ("CLB2PHY_TXBIT_TRI_RST_B_SMX0", SitePinDir::Out),
            ("CLB2PHY_TXBIT_TRI_RST_B_SMX1", SitePinDir::Out),
        ]);
    } else {
        pins.extend([
            // to BSCTL
            ("CLB2PHY_CTRL_RST_LOW_SMX", SitePinDir::Out),
            ("CLB2PHY_CTRL_RST_UPP_SMX", SitePinDir::Out),
            // to BITSLICE_TX
            ("CLB2PHY_TRISTATE_ODELAY_RST_SMX0", SitePinDir::Out),
            ("CLB2PHY_TRISTATE_ODELAY_RST_SMX1", SitePinDir::Out),
            ("CLB2PHY_TXBIT_TRI_RST_SMX0", SitePinDir::Out),
            ("CLB2PHY_TXBIT_TRI_RST_SMX1", SitePinDir::Out),
        ]);
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("DIV_CLK_OUT_LOW"),
        obel_bsctl_l.wire("LOCAL_DIV_CLK"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("DIV_CLK_OUT_UPP"),
        obel_bsctl_u.wire("LOCAL_DIV_CLK"),
    );
    let mut opins = vec![];
    // to BITSLICE_RX_TX
    for i in 0..13 {
        opins.push(format!("CLB2PHY_FIFO_CLK_SMX{i}"));
        if endev.edev.kind == ChipKind::Ultrascale {
            opins.push(format!("CLB2PHY_TXBIT_RST_B_SMX{i}"));
            opins.push(format!("CLB2PHY_RXBIT_RST_B_SMX{i}"));
            opins.push(format!("CLB2PHY_IDELAY_RST_B_SMX{i}"));
            opins.push(format!("CLB2PHY_ODELAY_RST_B_SMX{i}"));
        } else {
            opins.push(format!("CLB2PHY_TXBIT_RST_SMX{i}"));
            opins.push(format!("CLB2PHY_RXBIT_RST_SMX{i}"));
            opins.push(format!("CLB2PHY_IDELAY_RST_SMX{i}"));
            opins.push(format!("CLB2PHY_ODELAY_RST_SMX{i}"));
        }
    }
    for i in 0..6 {
        opins.push(format!("CTL2BS_REFCLK_EN_LOW_SMX{i}"));
    }
    for i in 0..7 {
        opins.push(format!("CTL2BS_REFCLK_EN_UPP_SMX{i}"));
    }
    let mut ipins = vec![];
    // from BSCTL
    for i in 0..7 {
        let pin_l = format!("CTL2BS_REFCLK_EN_LOW{i}");
        let pin_u = format!("CTL2BS_REFCLK_EN_UPP{i}");
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&pin_l),
            obel_bsctl_l.wire(&format!("REFCLK_EN{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&pin_u),
            obel_bsctl_u.wire(&format!("REFCLK_EN{i}")),
        );
        ipins.push(pin_l);
        ipins.push(pin_u);
    }
    for pin in &ipins {
        pins.push((pin, SitePinDir::In));
    }
    for pin in &opins {
        pins.push((pin, SitePinDir::Out));
    }
    vrf.verify_bel(bel, "XIPHY_FEEDTHROUGH", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_xiphy_byte(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let srow = chip.row_rclk(bel.row);
    let obel = vrf.get_bel(bel.cell.with_row(srow).bel(bels::RCLK_XIPHY));
    let bt = if bel.row < srow { 'B' } else { 'T' };
    for i in 0..6 {
        vrf.verify_node(&[
            bel.fwire(&format!("XIPHY_CLK{i}")),
            obel.fwire(&format!("XIPHY_CLK{i}_{bt}")),
        ]);
    }
}

fn verify_rclk_xiphy(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_RCLK_XIPHY);
    let side = endev.edev.chips[bel.die].col_side(bel.col);
    let hd_lr = match side {
        DirH::W => 'R',
        DirH::E => 'L',
    };
    for i in 0..6 {
        for bt in ['B', 'T'] {
            let pin = format!("XIPHY_CLK{i}_{bt}");
            vrf.claim_node(&[bel.fwire(&pin)]);
            vrf.claim_pip(bel.crd(), bel.wire(&pin), obel_vcc.wire("VCC"));
            for j in 0..24 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&pin),
                    bel.wire(&format!("HDISTR{j}_{hd_lr}")),
                );
            }
        }
    }
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            bel.wire(&format!("HDISTR{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            bel.wire(&format!("HDISTR{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
    }
    if side == DirH::W {
        let obel_hd = find_hdistr_src(endev, vrf, bel.cell);
        for i in 0..24 {
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_R")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
        }
    } else {
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_R"))]);
        }
    }
}

fn verify_hpiob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let idx = bels::HPIOB.into_iter().position(|x| bel.slot == x).unwrap();
    let pidx = if matches!(idx, 12 | 25) {
        None
    } else if idx < 12 {
        Some(idx ^ 1)
    } else {
        Some(((idx - 1) ^ 1) + 1)
    };
    let is_single = pidx.is_none();
    let is_m = !is_single
        && if idx < 13 {
            idx.is_multiple_of(2)
        } else {
            !idx.is_multiple_of(2)
        };
    let kind = if endev.edev.kind == ChipKind::Ultrascale {
        "HPIOB"
    } else if is_single {
        "HPIOB_SNGL"
    } else if is_m {
        "HPIOB_M"
    } else {
        "HPIOB_S"
    };
    let fidx = if bel.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG) {
        idx
    } else {
        idx + 26
    };
    let hid = TileIobId::from_idx(fidx);
    let reg = chip.row_to_reg(bel.row);

    let mut pins = vec![
        // to/from PHY
        ("I", SitePinDir::Out),
        ("OP", SitePinDir::In),
        ("TSP", SitePinDir::In),
        ("DYNAMIC_DCI_TS", SitePinDir::In),
        // to AMS
        ("SWITCH_OUT", SitePinDir::Out),
        // to/from paired IOB
        ("OUTB_B_IN", SitePinDir::In),
        ("OUTB_B", SitePinDir::Out),
        ("TSTATE_IN", SitePinDir::In),
        ("TSTATE_OUT", SitePinDir::Out),
        // to/from differential out
        ("O_B", SitePinDir::Out),
        ("TSTATEB", SitePinDir::Out),
        ("IO", SitePinDir::In),
        // from differential in
        ("LVDS_TRUE", SitePinDir::In),
        // from VREF
        ("VREF", SitePinDir::In),
        // dummies
        ("DOUT", SitePinDir::Out),
        ("TSDI", SitePinDir::In),
    ];

    // to differential in
    if endev.edev.kind == ChipKind::Ultrascale {
        pins.push(("CTLE_IN", SitePinDir::Out));
    }
    if endev.edev.kind == ChipKind::Ultrascale || !matches!(idx, 12 | 25) {
        pins.push(("PAD_RES", SitePinDir::Out));
    }

    let mut dummies = vec![];
    if is_single {
        dummies.push("IO");
        if endev.edev.kind == ChipKind::UltrascalePlus {
            dummies.push("LVDS_TRUE");
            dummies.push("OUTB_B_IN");
            dummies.push("TSTATE_IN");
        }
    }
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::HpioIob(bel.die, bel.col, reg, hid))
    {
        vrf.verify_bel_dummies(bel, kind, &pins, &[], &dummies);
    }
    for (pin, _) in pins {
        if !dummies.contains(&pin) && pin != "TSDI" {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }

    if let Some(pidx) = pidx {
        let obel = vrf.find_bel_sibling(bel, bels::HPIOB[pidx]);
        vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), obel.wire("OUTB_B"));
        vrf.claim_pip(bel.crd(), bel.wire("TSTATE_IN"), obel.wire("TSTATE_OUT"));
    } else if endev.edev.kind == ChipKind::Ultrascale {
        vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), bel.wire("OUTB_B"));
    }

    let obel_bs = if endev.edev.kind == ChipKind::Ultrascale {
        let srow = chip.row_rclk(bel.row);
        vrf.get_bel(
            bel.cell
                .with_row(srow)
                .bel(bels::BITSLICE[if bel.row < srow { idx } else { idx + 26 }]),
        )
    } else {
        let srow = if idx < 13 { bel.row } else { bel.row + 15 };
        vrf.get_bel(bel.cell.with_row(srow).bel(bels::BITSLICE[idx % 13]))
    };

    vrf.claim_pip(bel.crd(), bel.wire("OP"), bel.wire_far("OP"));
    vrf.claim_pip(bel.crd(), bel.wire("TSP"), bel.wire_far("TSP"));
    vrf.verify_node(&[bel.fwire("DYNAMIC_DCI_TS"), obel_bs.fwire("DYN_DCI_OUT")]);
    vrf.verify_node(&[bel.fwire("I"), obel_bs.fwire_far("RX_D")]);
    vrf.verify_node(&[bel.fwire_far("OP"), obel_bs.fwire("TX_Q")]);
    vrf.verify_node(&[bel.fwire_far("TSP"), obel_bs.fwire("TX_T_OUT")]);

    let crd = IoCoord::Hpio(HpioCoord {
        col: bel.col,
        die: bel.die,
        reg,
        iob: hid,
    });
    let cfg = endev.edev.cfg_io[bel.die].get_by_right(&crd);
    let is_cfg = match cfg {
        Some(SharedCfgPad::PerstN0) => false,
        Some(SharedCfgPad::PerstN1) => false,
        Some(SharedCfgPad::EmCclk) => false,
        Some(_) => true,
        None => false,
    };
    let is_ams = matches!(cfg, Some(SharedCfgPad::I2cSda | SharedCfgPad::I2cSclk))
        && endev.edev.kind == ChipKind::Ultrascale;
    if is_ams {
        vrf.claim_node(&[bel.fwire("TSDI")]);
        vrf.claim_pip(bel.crd(), bel.wire("TSDI"), bel.wire_far("TSDI"));
    } else if endev.edev.is_cut {
        if !(endev.edev.is_cut_d
            && endev.edev.kind == ChipKind::UltrascalePlus
            && bel.wire("TSDI") == bel.wire_far("TSDI")
            && is_cfg)
        {
            vrf.claim_node(&[bel.fwire("TSDI")]);
        }
    } else if !is_cfg {
        vrf.claim_node(&[bel.fwire("TSDI")]);
        if bel.wire("TSDI") != bel.wire_far("TSDI") {
            vrf.claim_pip(bel.crd(), bel.wire("TSDI"), bel.wire_far("TSDI"));
            vrf.claim_node(&[bel.fwire_far("TSDI")]);
        }
    } else if bel.wire("TSDI") != bel.wire_far("TSDI") {
        vrf.claim_node(&[bel.fwire("TSDI")]);
    }

    if !bel.naming.pins["SWITCH_OUT"].pips.is_empty() && !chip.config_kind.is_csec() {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("SWITCH_OUT"),
            bel.wire("SWITCH_OUT"),
        );

        let ams_idx = match fidx {
            4 | 5 => Some(15),
            6 | 7 => Some(7),
            8 | 9 => Some(14),
            10 | 11 => Some(6),
            13 | 14 => Some(13),
            15 | 16 => Some(5),
            17 | 18 => Some(12),
            19 | 20 => Some(4),
            30 | 31 => Some(11),
            32 | 33 => Some(3),
            34 | 35 => Some(10),
            36 | 37 => Some(2),
            39 | 40 => Some(9),
            41 | 42 => Some(1),
            43 | 44 => Some(8),
            45 | 46 => Some(0),
            _ => None,
        };

        if let Some(ams_idx) = ams_idx {
            let scol = chip.col_cfg();
            let srow = chip.row_ams();
            let obel = vrf.get_bel(bel.cell.with_cr(scol, srow).bel(bels::SYSMON));
            vrf.verify_node(&[
                bel.fwire_far("SWITCH_OUT"),
                obel.fwire_far(&format!(
                    "V{pn}_AUX{ams_idx}",
                    pn = if is_m { 'P' } else { 'N' }
                )),
            ]);
        }
    }

    let obel_vref = vrf.find_bel_sibling(bel, bels::HPIO_VREF);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("VREF"),
        obel_vref.wire(if idx < 13 { "VREF1" } else { "VREF2" }),
    );
}

fn verify_hpiodiffin(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let idx = bels::HPIOB_DIFF_IN
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let (pidx, nidx) = if idx < 6 {
        (idx * 2, idx * 2 + 1)
    } else {
        (idx * 2 + 1, idx * 2 + 2)
    };
    let reg = chip.row_to_reg(bel.row);
    let mut disabled = false;
    for sidx in [pidx, nidx] {
        let fidx = if bel.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG) {
            sidx
        } else {
            sidx + 26
        };
        let hid = TileIobId::from_idx(fidx);
        disabled |= endev
            .edev
            .disabled
            .contains(&DisabledPart::HpioIob(bel.die, bel.col, reg, hid));
    }
    let mut pins = vec![
        ("PAD_RES_0", SitePinDir::In),
        ("PAD_RES_1", SitePinDir::In),
        ("LVDS_TRUE", SitePinDir::Out),
        ("LVDS_COMP", SitePinDir::Out),
        ("VREF", SitePinDir::In),
    ];
    if endev.edev.kind == ChipKind::Ultrascale {
        pins.push(("CTLE_IN_1", SitePinDir::In));
    }
    if !disabled {
        vrf.verify_bel(bel, "HPIOBDIFFINBUF", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_p = vrf.find_bel_sibling(bel, bels::HPIOB[pidx]);
    let obel_n = vrf.find_bel_sibling(bel, bels::HPIOB[nidx]);
    vrf.claim_pip(bel.crd(), bel.wire("PAD_RES_0"), obel_p.wire("PAD_RES"));
    vrf.claim_pip(bel.crd(), bel.wire("PAD_RES_1"), obel_n.wire("PAD_RES"));
    vrf.claim_pip(bel.crd(), obel_p.wire("LVDS_TRUE"), bel.wire("LVDS_TRUE"));
    vrf.claim_pip(bel.crd(), obel_n.wire("LVDS_TRUE"), bel.wire("LVDS_COMP"));
    if endev.edev.kind == ChipKind::Ultrascale {
        vrf.claim_pip(bel.crd(), bel.wire("CTLE_IN_1"), obel_n.wire("CTLE_IN"));
    }

    let obel_vref = vrf.find_bel_sibling(bel, bels::HPIO_VREF);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("VREF"),
        obel_vref.wire(if idx < 6 { "VREF1" } else { "VREF2" }),
    );
}

fn verify_hpiodiffout(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let idx = bels::HPIOB_DIFF_OUT
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let (pidx, nidx) = if idx < 6 {
        (idx * 2, idx * 2 + 1)
    } else {
        (idx * 2 + 1, idx * 2 + 2)
    };
    let reg = chip.row_to_reg(bel.row);
    let mut disabled = false;
    for sidx in [pidx, nidx] {
        let fidx = if bel.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG) {
            sidx
        } else {
            sidx + 26
        };
        let hid = TileIobId::from_idx(fidx);
        disabled |= endev
            .edev
            .disabled
            .contains(&DisabledPart::HpioIob(bel.die, bel.col, reg, hid));
    }
    let pins = vec![
        ("O_B", SitePinDir::In),
        ("TSTATEB", SitePinDir::In),
        ("AOUT", SitePinDir::Out),
        ("BOUT", SitePinDir::Out),
    ];
    if !disabled {
        vrf.verify_bel(bel, "HPIOBDIFFOUTBUF", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_p = vrf.find_bel_sibling(bel, bels::HPIOB[pidx]);
    let obel_n = vrf.find_bel_sibling(bel, bels::HPIOB[nidx]);
    vrf.claim_pip(bel.crd(), bel.wire("O_B"), obel_p.wire("O_B"));
    vrf.claim_pip(bel.crd(), bel.wire("TSTATEB"), obel_p.wire("TSTATEB"));
    vrf.claim_pip(bel.crd(), obel_p.wire("IO"), bel.wire("AOUT"));
    vrf.claim_pip(bel.crd(), obel_n.wire("IO"), bel.wire("BOUT"));
}

fn verify_hpio_vref(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("VREF1", SitePinDir::Out), ("VREF2", SitePinDir::Out)];
    vrf.verify_bel(bel, "HPIO_VREF_SITE", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_hpio_dci(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let reg = chip.row_to_reg(bel.row);
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::HpioDci(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, "HPIOB_DCI_SNGL", &[], &[]);
    }
}

fn verify_hriob(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let idx = bels::HRIOB.into_iter().position(|x| bel.slot == x).unwrap();
    let pidx = if matches!(idx, 12 | 25) {
        None
    } else if idx < 12 {
        Some(idx ^ 1)
    } else {
        Some(((idx - 1) ^ 1) + 1)
    };
    let is_single = pidx.is_none();
    let is_m = !is_single
        && if idx < 13 {
            idx.is_multiple_of(2)
        } else {
            !idx.is_multiple_of(2)
        };
    let fidx = if bel.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG) {
        idx
    } else {
        idx + 26
    };
    let hid = TileIobId::from_idx(fidx);
    let reg = chip.row_to_reg(bel.row);
    let pins = vec![
        // to/from PHY
        ("DOUT", SitePinDir::Out),
        ("OP", SitePinDir::In),
        ("TSP", SitePinDir::In),
        ("DYNAMIC_DCI_TS", SitePinDir::In),
        // to AMS
        ("SWITCH_OUT", SitePinDir::Out),
        // to/from pair
        ("OUTB_B_IN", SitePinDir::In),
        ("OUTB_B", SitePinDir::Out),
        ("TSTATEIN", SitePinDir::In),
        ("TSTATEOUT", SitePinDir::Out),
        // to/from differential out
        ("O_B", SitePinDir::Out),
        ("TSTATEB", SitePinDir::Out),
        ("IO", SitePinDir::In),
        // to/from differential in
        ("TMDS_IBUF_OUT", SitePinDir::In),
        ("DRIVER_BOT_IBUF", SitePinDir::Out),
        // dummies
        ("TSDI", SitePinDir::In),
    ];
    let mut dummies = vec![];
    if is_single {
        dummies.push("IO");
    }
    vrf.verify_bel_dummies(bel, "HRIO", &pins, &[], &dummies);
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }

    if let Some(pidx) = pidx {
        let obel = vrf.find_bel_sibling(bel, bels::HRIOB[pidx]);
        vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), obel.wire("OUTB_B"));
        vrf.claim_pip(bel.crd(), bel.wire("TSTATEIN"), obel.wire("TSTATEOUT"));
    } else {
        vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), bel.wire("OUTB_B"));
    }

    let srow = chip.row_rclk(bel.row);
    let obel_bs = vrf.get_bel(
        bel.cell
            .with_row(srow)
            .bel(bels::BITSLICE[if bel.row < srow { idx } else { idx + 26 }]),
    );

    vrf.claim_pip(bel.crd(), bel.wire("OP"), bel.wire_far("OP"));
    vrf.claim_pip(bel.crd(), bel.wire("TSP"), bel.wire_far("TSP"));
    vrf.verify_node(&[bel.fwire("DYNAMIC_DCI_TS"), obel_bs.fwire("DYN_DCI_OUT")]);
    vrf.verify_node(&[bel.fwire("DOUT"), obel_bs.fwire_far("RX_D")]);
    vrf.verify_node(&[bel.fwire_far("OP"), obel_bs.fwire("TX_Q")]);
    vrf.verify_node(&[bel.fwire_far("TSP"), obel_bs.fwire("TX_T_OUT")]);

    let crd = IoCoord::Hpio(HpioCoord {
        col: bel.col,
        die: bel.die,
        reg,
        iob: hid,
    });
    let cfg = endev.edev.cfg_io[bel.die].get_by_right(&crd);
    let is_cfg = match cfg {
        Some(SharedCfgPad::PerstN0) => false,
        Some(SharedCfgPad::PerstN1) => false,
        Some(SharedCfgPad::EmCclk) => false,
        Some(_) => true,
        None => false,
    };
    let is_ams = matches!(cfg, Some(SharedCfgPad::I2cSda | SharedCfgPad::I2cSclk))
        && endev.edev.kind == ChipKind::Ultrascale;
    if is_ams {
        vrf.claim_pip(bel.crd(), bel.wire("TSDI"), bel.wire_far("TSDI"));
    } else if !endev.edev.is_cut && !is_cfg && bel.wire("TSDI") != bel.wire_far("TSDI") {
        vrf.claim_pip(bel.crd(), bel.wire("TSDI"), bel.wire_far("TSDI"));
        vrf.claim_node(&[bel.fwire_far("TSDI")]);
    }

    if !bel.naming.pins["SWITCH_OUT"].pips.is_empty() {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("SWITCH_OUT"),
            bel.wire("SWITCH_OUT"),
        );
        let fidx = if bel.row.to_idx().is_multiple_of(Chip::ROWS_PER_REG) {
            idx
        } else {
            idx + 26
        };

        let ams_idx = match fidx {
            4 | 5 => Some(15),
            6 | 7 => Some(7),
            8 | 9 => Some(14),
            10 | 11 => Some(6),
            13 | 14 => Some(13),
            15 | 16 => Some(5),
            17 | 18 => Some(12),
            19 | 20 => Some(4),
            30 | 31 => Some(11),
            32 | 33 => Some(3),
            34 | 35 => Some(10),
            36 | 37 => Some(2),
            39 | 40 => Some(9),
            41 | 42 => Some(1),
            43 | 44 => Some(8),
            45 | 46 => Some(0),
            _ => None,
        };

        if let Some(ams_idx) = ams_idx {
            let scol = chip.col_cfg();
            let srow = chip.row_ams();
            let obel = vrf.get_bel(bel.cell.with_cr(scol, srow).bel(bels::SYSMON));
            vrf.verify_node(&[
                bel.fwire_far("SWITCH_OUT"),
                obel.fwire_far(&format!(
                    "V{pn}_AUX{ams_idx}",
                    pn = if is_m { 'P' } else { 'N' }
                )),
            ]);
        }
    }
}

fn verify_hriodiffin(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::HRIOB_DIFF_IN
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let (pidx, nidx) = if idx < 6 {
        (idx * 2, idx * 2 + 1)
    } else {
        (idx * 2 + 1, idx * 2 + 2)
    };
    let pins = vec![
        ("LVDS_IN_P", SitePinDir::In),
        ("LVDS_IN_N", SitePinDir::In),
        ("LVDS_IBUF_OUT", SitePinDir::Out),
        ("LVDS_IBUF_OUT_B", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "HRIODIFFINBUF", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_p = vrf.find_bel_sibling(bel, bels::HRIOB[pidx]);
    let obel_n = vrf.find_bel_sibling(bel, bels::HRIOB[nidx]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("LVDS_IN_P"),
        obel_p.wire("DRIVER_BOT_IBUF"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("LVDS_IN_N"),
        obel_n.wire("DRIVER_BOT_IBUF"),
    );
    vrf.claim_pip(
        bel.crd(),
        obel_p.wire("TMDS_IBUF_OUT"),
        bel.wire("LVDS_IBUF_OUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        obel_n.wire("TMDS_IBUF_OUT"),
        bel.wire("LVDS_IBUF_OUT_B"),
    );
}

fn verify_hriodiffout(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx = bels::HRIOB_DIFF_OUT
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let (pidx, nidx) = if idx < 6 {
        (idx * 2, idx * 2 + 1)
    } else {
        (idx * 2 + 1, idx * 2 + 2)
    };
    let pins = vec![
        ("O_B", SitePinDir::In),
        ("TSTATEB", SitePinDir::In),
        ("AOUT", SitePinDir::Out),
        ("BOUT", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "HRIODIFFOUTBUF", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_p = vrf.find_bel_sibling(bel, bels::HRIOB[pidx]);
    let obel_n = vrf.find_bel_sibling(bel, bels::HRIOB[nidx]);
    vrf.claim_pip(bel.crd(), bel.wire("O_B"), obel_p.wire("O_B"));
    vrf.claim_pip(bel.crd(), bel.wire("TSTATEB"), obel_p.wire("TSTATEB"));
    vrf.claim_pip(bel.crd(), obel_p.wire("IO"), bel.wire("AOUT"));
    vrf.claim_pip(bel.crd(), obel_n.wire("IO"), bel.wire("BOUT"));
}

fn verify_bufg_gt(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let reg = chip.row_to_reg(bel.row);
    let mut pins = vec![
        ("CLK_IN", SitePinDir::In),
        ("CE", SitePinDir::In),
        ("RST_PRE_OPTINV", SitePinDir::In),
        ("CLK_OUT", SitePinDir::Out),
    ];
    if !bel.info.pins.contains_key("DIV0") {
        pins.extend([
            ("DIV0", SitePinDir::In),
            ("DIV1", SitePinDir::In),
            ("DIV2", SitePinDir::In),
        ]);
    }
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::GtBufs(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, "BUFG_GT", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_GT);
    if endev.edev.kind == ChipKind::Ultrascale {
        let (common, channel) = match bel.tcls {
            "GTH" => (bels::GTH_COMMON, bels::GTH_CHANNEL),
            "GTY" => (bels::GTY_COMMON, bels::GTY_CHANNEL),
            _ => unreachable!(),
        };
        for (slot, pin) in [
            (common, "REFCLK2HROW0"),
            (channel[0], "TXOUTCLK_INT"),
            (channel[1], "TXOUTCLK_INT"),
            (channel[0], "RXRECCLK_INT"),
            (channel[1], "RXRECCLK_INT"),
            (common, "REFCLK2HROW1"),
            (channel[2], "TXOUTCLK_INT"),
            (channel[3], "TXOUTCLK_INT"),
            (channel[2], "RXRECCLK_INT"),
            (channel[3], "RXRECCLK_INT"),
        ] {
            let obel = vrf.find_bel_sibling(bel, slot);
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
        }
        let obel = vrf.find_bel_sibling(bel, bels::BUFG_GT_SYNC10);
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_IN"));
        for i in 0..11 {
            let obel = vrf.find_bel_sibling(bel, bels::BUFG_GT_SYNC[i]);
            vrf.claim_pip(bel.crd(), bel.wire("CE"), obel.wire("CE_OUT"));
            vrf.claim_pip(bel.crd(), bel.wire("RST_PRE_OPTINV"), obel.wire("RST_OUT"));
        }
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("CE"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("RST_PRE_OPTINV"), obel_vcc.wire("VCC"));
        for i in 0..5 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLK_IN"),
                bel.wire(&format!("CLK_IN_MUX_DUMMY{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CE"),
                bel.wire(&format!("CE_MUX_DUMMY{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("RST_PRE_OPTINV"),
                bel.wire(&format!("RST_MUX_DUMMY{i}")),
            );
            vrf.claim_node(&[bel.fwire(&format!("CLK_IN_MUX_DUMMY{i}"))]);
            vrf.claim_node(&[bel.fwire(&format!("CE_MUX_DUMMY{i}"))]);
            vrf.claim_node(&[bel.fwire(&format!("RST_MUX_DUMMY{i}"))]);
        }
    } else {
        if bel.tcls.starts_with("GTM") {
            let obel = vrf.find_bel_sibling(bel, bels::GTM_DUAL);
            for i in 0..6 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLK_IN"),
                    obel.wire(&format!("CLK_BUFGT_CLK_IN_BOT{i}")),
                );
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLK_IN"),
                    obel.wire(&format!("CLK_BUFGT_CLK_IN_TOP{i}")),
                );
            }
            for slot in [bels::BUFG_GT_SYNC6, bels::BUFG_GT_SYNC13] {
                let obel = vrf.find_bel_sibling(bel, slot);
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_IN"));
            }
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("CLK_IN_MUX_DUMMY0"));
            vrf.claim_pip(bel.crd(), bel.wire("CE"), bel.wire("CE_MUX_DUMMY0"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("RST_PRE_OPTINV"),
                bel.wire("RST_MUX_DUMMY0"),
            );
            vrf.claim_node(&[bel.fwire("CLK_IN_MUX_DUMMY0")]);
            vrf.claim_node(&[bel.fwire("CE_MUX_DUMMY0")]);
            vrf.claim_node(&[bel.fwire("RST_MUX_DUMMY0")]);
        } else if bel.tcls.starts_with("GT") {
            let (common, channel) = match bel.tcls {
                "GTH" => (bels::GTH_COMMON, bels::GTH_CHANNEL),
                "GTY" => (bels::GTY_COMMON, bels::GTY_CHANNEL),
                "GTF" => (bels::GTF_COMMON, bels::GTF_CHANNEL),
                _ => unreachable!(),
            };
            for (slot, pin) in [
                (common, "REFCLK2HROW0"),
                (channel[0], "TXOUTCLK_INT"),
                (channel[1], "TXOUTCLK_INT"),
                (channel[0], "RXRECCLK_INT"),
                (channel[1], "RXRECCLK_INT"),
                (channel[0], "DMONOUTCLK_INT"),
                (channel[1], "DMONOUTCLK_INT"),
                (common, "REFCLK2HROW1"),
                (channel[2], "TXOUTCLK_INT"),
                (channel[3], "TXOUTCLK_INT"),
                (channel[2], "RXRECCLK_INT"),
                (channel[3], "RXRECCLK_INT"),
                (channel[2], "DMONOUTCLK_INT"),
                (channel[3], "DMONOUTCLK_INT"),
            ] {
                let obel = vrf.find_bel_sibling(bel, slot);
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
            }
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("CLK_IN_MUX_DUMMY0"));
            vrf.claim_pip(bel.crd(), bel.wire("CE"), bel.wire("CE_MUX_DUMMY0"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("RST_PRE_OPTINV"),
                bel.wire("RST_MUX_DUMMY0"),
            );
            vrf.claim_node(&[bel.fwire("CLK_IN_MUX_DUMMY0")]);
            vrf.claim_node(&[bel.fwire("CE_MUX_DUMMY0")]);
            vrf.claim_node(&[bel.fwire("RST_MUX_DUMMY0")]);
        } else {
            let oslot = match bel.tcls {
                "HSADC" => bels::HSADC,
                "HSDAC" => bels::HSDAC,
                "RFADC" => bels::RFADC,
                "RFDAC" => bels::RFDAC,
                _ => unreachable!(),
            };
            let obel = vrf.find_bel_sibling(bel, oslot);
            if bel.tcls.ends_with("ADC") {
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_ADC"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_ADC_SPARE"));
            } else {
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_DAC"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_DAC_SPARE"));
            }
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("PLL_DMON_OUT"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("PLL_REFCLK_OUT"));
            for i in 0..11 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLK_IN"),
                    bel.wire(&format!("CLK_IN_MUX_DUMMY{i}")),
                );
                vrf.claim_node(&[bel.fwire(&format!("CLK_IN_MUX_DUMMY{i}"))]);
            }
            vrf.claim_pip(bel.crd(), bel.wire("CE"), bel.wire("CE_MUX_DUMMY0"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("RST_PRE_OPTINV"),
                bel.wire("RST_MUX_DUMMY0"),
            );
            vrf.claim_node(&[bel.fwire("CE_MUX_DUMMY0")]);
            vrf.claim_node(&[bel.fwire("RST_MUX_DUMMY0")]);
            vrf.claim_pip(bel.crd(), bel.wire("DIV0"), bel.wire("DIV0_DUMMY"));
            vrf.claim_pip(bel.crd(), bel.wire("DIV1"), bel.wire("DIV1_DUMMY"));
            vrf.claim_pip(bel.crd(), bel.wire("DIV2"), bel.wire("DIV2_DUMMY"));
            vrf.claim_node(&[bel.fwire("DIV0_DUMMY")]);
            vrf.claim_node(&[bel.fwire("DIV1_DUMMY")]);
            vrf.claim_node(&[bel.fwire("DIV2_DUMMY")]);
        }
        let obel = vrf.find_bel_sibling(bel, bels::BUFG_GT_SYNC14);
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_IN"));
        for i in 0..15 {
            let obel = vrf.find_bel_sibling(bel, bels::BUFG_GT_SYNC[i]);
            vrf.claim_pip(bel.crd(), bel.wire("CE"), obel.wire("CE_OUT"));
            vrf.claim_pip(bel.crd(), bel.wire("RST_PRE_OPTINV"), obel.wire("RST_OUT"));
        }
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("CE"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("RST_PRE_OPTINV"), obel_vcc.wire("VCC"));
    }
}

fn verify_bufg_gt_sync(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let idx = bels::BUFG_GT_SYNC
        .into_iter()
        .position(|x| bel.slot == x)
        .unwrap();
    let mut pins = vec![("CE_OUT", SitePinDir::Out), ("RST_OUT", SitePinDir::Out)];
    let mut dummies = vec![];
    let mut is_int = false;
    if endev.edev.kind == ChipKind::Ultrascale {
        if idx == 10 {
            is_int = true;
        } else {
            let (common, channel) = match bel.tcls {
                "GTH" => (bels::GTH_COMMON, bels::GTH_CHANNEL),
                "GTY" => (bels::GTY_COMMON, bels::GTY_CHANNEL),
                "GTF" => (bels::GTF_COMMON, bels::GTF_CHANNEL),
                _ => unreachable!(),
            };
            let (oslot, pin) = match idx {
                0 => (common, "REFCLK2HROW0"),
                1 => (channel[0], "TXOUTCLK_INT"),
                2 => (channel[1], "TXOUTCLK_INT"),
                3 => (channel[0], "RXRECCLK_INT"),
                4 => (channel[1], "RXRECCLK_INT"),
                5 => (common, "REFCLK2HROW1"),
                6 => (channel[2], "TXOUTCLK_INT"),
                7 => (channel[3], "TXOUTCLK_INT"),
                8 => (channel[2], "RXRECCLK_INT"),
                9 => (channel[3], "RXRECCLK_INT"),
                _ => unreachable!(),
            };
            let obel = vrf.find_bel_sibling(bel, oslot);
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
        }
    } else {
        if idx == 14 {
            is_int = true;
        } else {
            if bel.tcls.starts_with("GTM") {
                if matches!(idx, 6 | 13) {
                    dummies.push("CLK_IN");
                } else {
                    let obel = vrf.find_bel_sibling(bel, bels::GTM_DUAL);
                    let pin = if idx < 6 {
                        format!("CLK_BUFGT_CLK_IN_BOT{idx}")
                    } else {
                        format!("CLK_BUFGT_CLK_IN_TOP{ii}", ii = idx - 7)
                    };
                    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(&pin));
                }
            } else if bel.tcls.starts_with("GT") {
                let (common, channel) = match bel.tcls {
                    "GTH" => (bels::GTH_COMMON, bels::GTH_CHANNEL),
                    "GTY" => (bels::GTY_COMMON, bels::GTY_CHANNEL),
                    "GTF" => (bels::GTF_COMMON, bels::GTF_CHANNEL),
                    _ => unreachable!(),
                };
                let (oslot, pin) = match idx {
                    0 => (common, "REFCLK2HROW0"),
                    1 => (channel[0], "TXOUTCLK_INT"),
                    2 => (channel[1], "TXOUTCLK_INT"),
                    3 => (channel[0], "RXRECCLK_INT"),
                    4 => (channel[1], "RXRECCLK_INT"),
                    5 => (channel[0], "DMONOUTCLK_INT"),
                    6 => (channel[1], "DMONOUTCLK_INT"),
                    7 => (common, "REFCLK2HROW1"),
                    8 => (channel[2], "TXOUTCLK_INT"),
                    9 => (channel[3], "TXOUTCLK_INT"),
                    10 => (channel[2], "RXRECCLK_INT"),
                    11 => (channel[3], "RXRECCLK_INT"),
                    12 => (channel[2], "DMONOUTCLK_INT"),
                    13 => (channel[3], "DMONOUTCLK_INT"),
                    _ => unreachable!(),
                };
                let obel = vrf.find_bel_sibling(bel, oslot);
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
            } else {
                if idx < 4 {
                    let is_adc = bel.tcls.ends_with("ADC");
                    let pin = match (idx, is_adc) {
                        (0, true) => "CLK_ADC",
                        (0, false) => "CLK_DAC",
                        (1, _) => "PLL_DMON_OUT",
                        (2, _) => "PLL_REFCLK_OUT",
                        (3, true) => "CLK_ADC_SPARE",
                        (3, false) => "CLK_DAC_SPARE",
                        _ => unreachable!(),
                    };
                    let oslot = match bel.tcls {
                        "HSADC" => bels::HSADC,
                        "HSDAC" => bels::HSDAC,
                        "RFADC" => bels::RFADC,
                        "RFDAC" => bels::RFDAC,
                        _ => unreachable!(),
                    };
                    let obel = vrf.find_bel_sibling(bel, oslot);
                    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
                }
            }
        }
        if !bel.info.pins.contains_key("CE_IN") {
            pins.extend([("CE_IN", SitePinDir::In), ("RST_IN", SitePinDir::In)]);
        }
    }
    if !is_int {
        pins.push(("CLK_IN", SitePinDir::In));
    }

    let reg = chip.row_to_reg(bel.row);
    let skip = (endev
        .edev
        .disabled
        .contains(&DisabledPart::GtmSpareBufs(bel.die, bel.col, reg))
        && matches!(idx, 6 | 13))
        || endev
            .edev
            .disabled
            .contains(&DisabledPart::GtBufs(bel.die, bel.col, reg));
    if !skip {
        vrf.verify_bel_dummies(bel, "BUFG_GT_SYNC", &pins, &[], &dummies);
    }
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
}

fn verify_gt_channel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let (common, channel) = match bel.tcls {
        "GTH" => (bels::GTH_COMMON, bels::GTH_CHANNEL),
        "GTY" => (bels::GTY_COMMON, bels::GTY_CHANNEL),
        "GTF" => (bels::GTF_COMMON, bels::GTF_CHANNEL),
        _ => unreachable!(),
    };
    let idx = channel.into_iter().position(|x| x == bel.slot).unwrap();
    let chip = endev.edev.chips[bel.die];
    let kind = match bel.tcls {
        "GTH" => match endev.edev.kind {
            ChipKind::Ultrascale => "GTHE3_CHANNEL",
            ChipKind::UltrascalePlus => "GTHE4_CHANNEL",
        },
        "GTY" => match endev.edev.kind {
            ChipKind::Ultrascale => "GTYE3_CHANNEL",
            ChipKind::UltrascalePlus => "GTYE4_CHANNEL",
        },
        "GTF" => "GTF_CHANNEL",
        _ => unreachable!(),
    };
    let mut pins = vec![
        // from COMMON
        ("MGTREFCLK0", SitePinDir::In),
        ("MGTREFCLK1", SitePinDir::In),
        ("NORTHREFCLK0", SitePinDir::In),
        ("NORTHREFCLK1", SitePinDir::In),
        ("SOUTHREFCLK0", SitePinDir::In),
        ("SOUTHREFCLK1", SitePinDir::In),
        ("QDCMREFCLK0_INT", SitePinDir::In),
        ("QDCMREFCLK1_INT", SitePinDir::In),
        ("QDPLL0CLK0P_INT", SitePinDir::In),
        ("QDPLL1CLK0P_INT", SitePinDir::In),
        ("RING_OSC_CLK_INT", SitePinDir::In),
        // to COMMON
        ("RXRECCLKOUT", SitePinDir::Out),
        // to BUFG_*
        ("RXRECCLK_INT", SitePinDir::Out),
        ("TXOUTCLK_INT", SitePinDir::Out),
    ];
    if endev.edev.kind == ChipKind::UltrascalePlus {
        // to BUFG_*
        pins.push(("DMONOUTCLK_INT", SitePinDir::Out));
    }
    let reg = chip.row_to_reg(bel.row);
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, kind, &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel = vrf.find_bel_sibling(bel, common);
    let cross_qdpll = endev.edev.kind == ChipKind::Ultrascale && bel.tcls.starts_with("GTH");
    for (pin, opin) in [
        ("MGTREFCLK0", "MGTREFCLK0"),
        ("MGTREFCLK1", "MGTREFCLK1"),
        ("NORTHREFCLK0", "NORTHREFCLK0"),
        ("NORTHREFCLK1", "NORTHREFCLK1"),
        ("SOUTHREFCLK0", "SOUTHREFCLK0"),
        ("SOUTHREFCLK1", "SOUTHREFCLK1"),
        ("QDCMREFCLK0_INT", "QDCMREFCLK_INT_0"),
        ("QDCMREFCLK1_INT", "QDCMREFCLK_INT_1"),
        (
            "QDPLL0CLK0P_INT",
            if cross_qdpll {
                "QDPLLCLK0P_1"
            } else {
                "QDPLLCLK0P_0"
            },
        ),
        (
            "QDPLL1CLK0P_INT",
            if cross_qdpll {
                "QDPLLCLK0P_0"
            } else {
                "QDPLLCLK0P_1"
            },
        ),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(opin));
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("RING_OSC_CLK_INT"),
        obel.wire(&format!("SARC_CLK{idx}")),
    );
}

fn verify_gt_common(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let channel = match bel.tcls {
        "GTH" => bels::GTH_CHANNEL,
        "GTY" => bels::GTY_CHANNEL,
        "GTF" => bels::GTF_CHANNEL,
        _ => unreachable!(),
    };

    let chip = endev.edev.chips[bel.die];
    let kind = match bel.tcls {
        "GTH" => match endev.edev.kind {
            ChipKind::Ultrascale => "GTHE3_COMMON",
            ChipKind::UltrascalePlus => "GTHE4_COMMON",
        },
        "GTY" => match endev.edev.kind {
            ChipKind::Ultrascale => "GTYE3_COMMON",
            ChipKind::UltrascalePlus => "GTYE4_COMMON",
        },
        "GTF" => "GTF_COMMON",
        _ => unreachable!(),
    };
    let pins = [
        // from CHANNEL
        ("RXRECCLK0", SitePinDir::In),
        ("RXRECCLK1", SitePinDir::In),
        ("RXRECCLK2", SitePinDir::In),
        ("RXRECCLK3", SitePinDir::In),
        // to CHANNEL, broadcast
        ("QDCMREFCLK_INT_0", SitePinDir::Out),
        ("QDCMREFCLK_INT_1", SitePinDir::Out),
        ("QDPLLCLK0P_0", SitePinDir::Out),
        ("QDPLLCLK0P_1", SitePinDir::Out),
        ("MGTREFCLK0", SitePinDir::Out),
        ("MGTREFCLK1", SitePinDir::Out),
        // to CHANNEL, specific
        ("SARC_CLK0", SitePinDir::Out),
        ("SARC_CLK1", SitePinDir::Out),
        ("SARC_CLK2", SitePinDir::Out),
        ("SARC_CLK3", SitePinDir::Out),
        // to BUFG
        ("REFCLK2HROW0", SitePinDir::Out),
        ("REFCLK2HROW1", SitePinDir::Out),
        // from self and up/down
        ("COM0_REFCLKOUT0", SitePinDir::In),
        ("COM0_REFCLKOUT1", SitePinDir::In),
        ("COM0_REFCLKOUT2", SitePinDir::In),
        ("COM0_REFCLKOUT3", SitePinDir::In),
        ("COM0_REFCLKOUT4", SitePinDir::In),
        ("COM0_REFCLKOUT5", SitePinDir::In),
        ("COM2_REFCLKOUT0", SitePinDir::In),
        ("COM2_REFCLKOUT1", SitePinDir::In),
        ("COM2_REFCLKOUT2", SitePinDir::In),
        ("COM2_REFCLKOUT3", SitePinDir::In),
        ("COM2_REFCLKOUT4", SitePinDir::In),
        ("COM2_REFCLKOUT5", SitePinDir::In),
    ];
    let reg = chip.row_to_reg(bel.row);
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, kind, &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    for i in 0..4 {
        let obel = vrf.find_bel_sibling(bel, channel[i]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXRECCLK{i}")),
            obel.wire("RXRECCLKOUT"),
        );
    }

    for (i, pin) in [
        (0, "MGTREFCLK0"),
        (1, "MGTREFCLK1"),
        (2, "NORTHREFCLK0"),
        (3, "NORTHREFCLK1"),
        (4, "SOUTHREFCLK0"),
        (5, "SOUTHREFCLK1"),
    ] {
        for j in [0, 2] {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("COM{j}_REFCLKOUT{i}")),
                bel.wire(pin),
            );
        }
    }

    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_GT);
    for pin in [
        "CLKOUT_NORTH0",
        "CLKOUT_NORTH1",
        "CLKOUT_SOUTH0",
        "CLKOUT_SOUTH1",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("MGTREFCLK0"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("MGTREFCLK1"));
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_NORTH0"),
        bel.wire("NORTHREFCLK0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_NORTH1"),
        bel.wire("NORTHREFCLK1"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_SOUTH0"),
        bel.wire("SOUTHREFCLK0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_SOUTH1"),
        bel.wire("SOUTHREFCLK1"),
    );

    if let Some(obel_n) = vrf.find_bel_delta(bel, 0, Chip::ROWS_PER_REG as isize, bel.slot) {
        vrf.verify_node(&[bel.fwire("SOUTHREFCLK0"), obel_n.fwire("CLKOUT_SOUTH0")]);
        vrf.verify_node(&[bel.fwire("SOUTHREFCLK1"), obel_n.fwire("CLKOUT_SOUTH1")]);
        vrf.claim_node(&[bel.fwire("CLKOUT_NORTH0")]);
        vrf.claim_node(&[bel.fwire("CLKOUT_NORTH1")]);
    } else {
        vrf.claim_dummy_in(bel.fwire("SOUTHREFCLK0"));
        vrf.claim_dummy_in(bel.fwire("SOUTHREFCLK1"));
        vrf.claim_dummy_out(bel.fwire("CLKOUT_NORTH0"));
        vrf.claim_dummy_out(bel.fwire("CLKOUT_NORTH1"));
    }
    if let Some(obel_s) = vrf.find_bel_delta(bel, 0, -(Chip::ROWS_PER_REG as isize), bel.slot) {
        vrf.verify_node(&[bel.fwire("NORTHREFCLK0"), obel_s.fwire("CLKOUT_NORTH0")]);
        vrf.verify_node(&[bel.fwire("NORTHREFCLK1"), obel_s.fwire("CLKOUT_NORTH1")]);
        vrf.claim_node(&[bel.fwire("CLKOUT_SOUTH0")]);
        vrf.claim_node(&[bel.fwire("CLKOUT_SOUTH1")]);
    } else {
        vrf.claim_dummy_in(bel.fwire("NORTHREFCLK0"));
        vrf.claim_dummy_in(bel.fwire("NORTHREFCLK1"));
        vrf.claim_dummy_out(bel.fwire("CLKOUT_SOUTH0"));
        vrf.claim_dummy_out(bel.fwire("CLKOUT_SOUTH1"));
    }
}

fn verify_gtm_dual(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let pins = [
        // BUFG_*
        ("CLK_BUFGT_CLK_IN_BOT0", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT1", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT2", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT3", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT4", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT5", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP0", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP1", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP2", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP3", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP4", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP5", SitePinDir::Out),
        // to/from GTM_REFCLK
        ("HROW_TEST_CK_SA", SitePinDir::Out),
        ("REFCLKPDB_SA", SitePinDir::Out),
        ("RXRECCLK0_INT", SitePinDir::Out),
        ("RXRECCLK1_INT", SitePinDir::Out),
        ("MGTREFCLK_CLEAN", SitePinDir::In),
        ("REFCLK2HROW", SitePinDir::In),
        // from s/n
        ("REFCLK_DIST2PLL0", SitePinDir::In),
        ("REFCLK_DIST2PLL1", SitePinDir::In),
        // dummy ins
        ("RCALSEL0", SitePinDir::In),
        ("RCALSEL1", SitePinDir::In),
    ];
    let reg = chip.row_to_reg(bel.row);
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, "GTM_DUAL", &pins, &[]);
    }
    for (pin, _) in pins {
        if pin == "REFCLK_DIST2PLL1" && is_cut_u(endev, bel.die, bel.row) {
            continue;
        }
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, bels::GTM_REFCLK);
    for (pin, opin) in [
        ("REFCLK2HROW", "REFCLK2HROW"),
        ("MGTREFCLK_CLEAN", "MGTREFCLK_CLEAN"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(opin));
    }
    if let Some(obel_n) = vrf.find_bel_delta(bel, 0, Chip::ROWS_PER_REG as isize, bel.slot) {
        vrf.verify_node(&[bel.fwire("REFCLK_DIST2PLL1"), obel_n.fwire("SOUTHCLKOUT")]);
    } else {
        vrf.claim_node(&[bel.fwire("NORTHCLKOUT")]);
    }
    if let Some(obel_s) = vrf.find_bel_delta(bel, 0, -(Chip::ROWS_PER_REG as isize), bel.slot) {
        vrf.verify_node(&[bel.fwire("REFCLK_DIST2PLL0"), obel_s.fwire("NORTHCLKOUT")]);
    } else {
        vrf.claim_node(&[bel.fwire("SOUTHCLKOUT")]);
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NORTHCLKOUT"),
        bel.wire("REFCLK_DIST2PLL0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NORTHCLKOUT"),
        obel.wire("MGTREFCLK_CLEAN"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NORTHCLKOUT"),
        bel.wire("NORTHCLKOUT_DUMMY0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NORTHCLKOUT"),
        bel.wire("NORTHCLKOUT_DUMMY1"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("SOUTHCLKOUT"),
        bel.wire("REFCLK_DIST2PLL1"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("SOUTHCLKOUT"),
        obel.wire("MGTREFCLK_CLEAN"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("SOUTHCLKOUT"),
        bel.wire("SOUTHCLKOUT_DUMMY0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("SOUTHCLKOUT"),
        bel.wire("SOUTHCLKOUT_DUMMY1"),
    );
    vrf.claim_node(&[bel.fwire("NORTHCLKOUT_DUMMY0")]);
    vrf.claim_node(&[bel.fwire("NORTHCLKOUT_DUMMY1")]);
    vrf.claim_node(&[bel.fwire("SOUTHCLKOUT_DUMMY0")]);
    vrf.claim_node(&[bel.fwire("SOUTHCLKOUT_DUMMY1")]);
}

fn verify_gtm_refclk(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let pins = [
        ("HROW_TEST_CK_FS", SitePinDir::In),
        ("REFCLKPDB_SA", SitePinDir::In),
        ("RXRECCLK0_INT", SitePinDir::In),
        ("RXRECCLK1_INT", SitePinDir::In),
        ("RXRECCLK2_INT", SitePinDir::In),
        ("RXRECCLK3_INT", SitePinDir::In),
        ("MGTREFCLK_CLEAN", SitePinDir::Out),
        ("REFCLK2HROW", SitePinDir::Out),
    ];
    let reg = chip.row_to_reg(bel.row);
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, "GTM_REFCLK", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, bels::GTM_DUAL);
    for (pin, opin) in [
        ("HROW_TEST_CK_FS", "HROW_TEST_CK_SA"),
        ("REFCLKPDB_SA", "REFCLKPDB_SA"),
        ("RXRECCLK0_INT", "RXRECCLK0_INT"),
        ("RXRECCLK1_INT", "RXRECCLK1_INT"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(opin));
    }
}

fn verify_hsadc_hsdac(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let chip = endev.edev.chips[bel.die];
    let slot_name = endev.edev.egrid.db.bel_slots.key(bel.slot);
    let mut pins = vec![
        // to/from north/south
        ("SYSREF_IN_SOUTH_P", SitePinDir::In),
        ("SYSREF_IN_NORTH_P", SitePinDir::In),
        ("SYSREF_OUT_SOUTH_P", SitePinDir::Out),
        ("SYSREF_OUT_NORTH_P", SitePinDir::Out),
        // to BUFG_*
        ("PLL_DMON_OUT", SitePinDir::Out),
        ("PLL_REFCLK_OUT", SitePinDir::Out),
    ];
    // to BUFG_*
    if slot_name.ends_with("ADC") {
        pins.extend([
            ("CLK_ADC", SitePinDir::Out),
            ("CLK_ADC_SPARE", SitePinDir::Out),
        ]);
    } else {
        pins.extend([
            ("CLK_DAC", SitePinDir::Out),
            ("CLK_DAC_SPARE", SitePinDir::Out),
        ]);
    }
    // to/from north/south
    if slot_name.starts_with("RF") {
        pins.extend([
            ("CLK_DISTR_IN_NORTH", SitePinDir::In),
            ("CLK_DISTR_IN_SOUTH", SitePinDir::In),
            ("CLK_DISTR_OUT_NORTH", SitePinDir::Out),
            ("CLK_DISTR_OUT_SOUTH", SitePinDir::Out),
            ("T1_ALLOWED_NORTH", SitePinDir::In),
            ("T1_ALLOWED_SOUTH", SitePinDir::Out),
        ]);
    }
    let reg = chip.row_to_reg(bel.row);
    if !endev
        .edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, slot_name, &pins, &[]);
    }
    for (pin, dir) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
        if dir == SitePinDir::In {
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
        }
    }

    let oslot = match bel.slot {
        bels::HSADC => bels::HSDAC,
        bels::HSDAC => bels::HSADC,
        bels::RFADC => bels::RFDAC,
        bels::RFDAC => bels::RFADC,
        _ => unreachable!(),
    };

    if let Some(obel_n) = vrf
        .find_bel_delta(bel, 0, Chip::ROWS_PER_REG as isize, bel.slot)
        .or_else(|| vrf.find_bel_delta(bel, 0, Chip::ROWS_PER_REG as isize, oslot))
    {
        vrf.verify_node(&[
            bel.fwire_far("SYSREF_IN_NORTH_P"),
            obel_n.fwire("SYSREF_OUT_SOUTH_P"),
        ]);
        if slot_name.starts_with("RF") {
            if obel_n.slot == bel.slot {
                vrf.verify_node(&[
                    bel.fwire_far("CLK_DISTR_IN_NORTH"),
                    obel_n.fwire("CLK_DISTR_OUT_SOUTH"),
                ]);
            }
            vrf.verify_node(&[
                bel.fwire_far("T1_ALLOWED_NORTH"),
                obel_n.fwire("T1_ALLOWED_SOUTH"),
            ]);
        }
    } else if chip.row_to_reg(bel.row).to_idx() == chip.regs - 1 {
        vrf.verify_node(&[
            bel.fwire_far("SYSREF_IN_NORTH_P"),
            bel.fwire("SYSREF_OUT_NORTH_P"),
        ]);
        if slot_name.starts_with("RF") {
            vrf.verify_node(&[
                bel.fwire_far("CLK_DISTR_IN_NORTH"),
                bel.fwire("CLK_DISTR_OUT_NORTH"),
            ]);
        }
    }
    if let Some(obel_s) = vrf
        .find_bel_delta(bel, 0, -(Chip::ROWS_PER_REG as isize), bel.slot)
        .or_else(|| vrf.find_bel_delta(bel, 0, -(Chip::ROWS_PER_REG as isize), oslot))
    {
        vrf.verify_node(&[
            bel.fwire_far("SYSREF_IN_SOUTH_P"),
            obel_s.fwire("SYSREF_OUT_NORTH_P"),
        ]);
        if slot_name.starts_with("RF") {
            if obel_s.slot == bel.slot {
                vrf.verify_node(&[
                    bel.fwire_far("CLK_DISTR_IN_SOUTH"),
                    obel_s.fwire("CLK_DISTR_OUT_NORTH"),
                ]);
            } else {
                vrf.verify_node(&[
                    bel.fwire_far("CLK_DISTR_IN_SOUTH"),
                    bel.fwire("T1_ALLOWED_SOUTH"),
                ]);
            }
        }
    } else {
        vrf.verify_node(&[
            bel.fwire_far("SYSREF_IN_SOUTH_P"),
            bel.fwire("SYSREF_OUT_SOUTH_P"),
        ]);
        if slot_name.starts_with("RF") {
            vrf.verify_node(&[
                bel.fwire_far("CLK_DISTR_IN_SOUTH"),
                bel.fwire("CLK_DISTR_OUT_SOUTH"),
            ]);
        }
    }
}

fn verify_rclk_gt(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = endev.edev.chips[bel.die].col_side(bel.col) == DirH::W;
    let obel_vcc = vrf.find_bel_sibling(bel, bels::VCC_GT);
    for i in 0..24 {
        let lr = if is_l { 'R' } else { 'L' };
        let hr = format!("HROUTE{i}_{lr}");
        let hd = format!("HDISTR{i}_{lr}");
        let obel = vrf.find_bel_sibling(bel, bels::BUFG_GT[i]);
        vrf.claim_pip(bel.crd(), bel.wire(&hr), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire(&hr), obel.wire("CLK_OUT"));
        vrf.claim_pip(bel.crd(), bel.wire(&hd), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire(&hd), obel.wire("CLK_OUT"));
    }
    if is_l {
        let obel_hd = find_hdistr_src(endev, vrf, bel.cell);
        let obel_hr = find_hroute_src(endev, vrf, bel.cell);
        for i in 0..24 {
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_R")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("HROUTE{i}_R")),
                obel_hr.fwire(&format!("HROUTE{i}_L")),
            ]);
        }
    } else {
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
            vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        }
    }
}

fn verify_bel(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let slot_name = endev.edev.egrid.db.bel_slots.key(bel.slot);
    match bel.slot {
        bels::SLICE => verify_slice(endev, vrf, bel),
        bels::DSP0 | bels::DSP1 => verify_dsp(endev, vrf, bel),
        bels::BRAM_F => verify_bram_f(endev, vrf, bel),
        bels::BRAM_H0 | bels::BRAM_H1 => verify_bram_h(vrf, bel),
        bels::HARD_SYNC0 | bels::HARD_SYNC1 | bels::HARD_SYNC2 | bels::HARD_SYNC3 => {
            vrf.verify_bel(bel, "HARD_SYNC", &[], &[])
        }
        bels::URAM0 | bels::URAM1 | bels::URAM2 | bels::URAM3 => verify_uram(endev, vrf, bel),
        bels::LAGUNA0 | bels::LAGUNA1 | bels::LAGUNA2 | bels::LAGUNA3 => {
            verify_laguna(endev, vrf, bel)
        }
        bels::LAGUNA_EXTRA => verify_laguna_extra(endev, vrf, bel),
        _ if slot_name.starts_with("VCC") => verify_vcc(vrf, bel),

        bels::PCIE3 | bels::PCIE4 | bels::PCIE4C | bels::PCIE4CE => verify_pcie(endev, vrf, bel),
        bels::CMAC => verify_cmac(endev, vrf, bel),
        bels::ILKN => verify_ilkn(endev, vrf, bel),
        bels::PMV
        | bels::PMV2
        | bels::PMVIOB
        | bels::MTBF3
        | bels::BLI_HBM_APB_INTF
        | bels::BLI_HBM_AXI_INTF
        | bels::HDIO_BIAS => vrf.verify_bel(bel, slot_name, &[], &[]),
        bels::CFGIO => vrf.verify_bel(bel, "CFGIO_SITE", &[], &[]),
        bels::HPIO_ZMATCH => vrf.verify_bel(bel, "HPIO_ZMATCH_BLK_HCLK", &[], &[]),
        bels::HPIO_PRBS => vrf.verify_bel(bel, "HPIO_RCLK_PRBS", &[], &[]),
        bels::HDLOGIC_CSSD0 | bels::HDLOGIC_CSSD1 | bels::HDLOGIC_CSSD2 | bels::HDLOGIC_CSSD3 => {
            let kind = if bel.tcls == "HDIOS" {
                if matches!(bel.slot, bels::HDLOGIC_CSSD2 | bels::HDLOGIC_CSSD3) {
                    "HDIOS_HDLOGIC_CSSD_TOP"
                } else {
                    "HDIOS_HDLOGIC_CSSD"
                }
            } else {
                "HDLOGIC_CSSD"
            };
            vrf.verify_bel(bel, kind, &[], &[])
        }
        bels::HDIO_VREF0 | bels::HDIO_VREF1 | bels::HDIO_VREF2 => {
            vrf.verify_bel(bel, "HDIO_VREF", &[], &[])
        }
        bels::FE => {
            if !endev.edev.disabled.contains(&DisabledPart::Sdfec) {
                vrf.verify_bel(bel, "FE", &[], &[]);
            }
        }
        bels::DFE_A => {
            let chip = endev.edev.chips[bel.die];
            let reg = chip.row_to_reg(bel.row);
            if !endev
                .edev
                .disabled
                .contains(&DisabledPart::HardIp(bel.die, bel.col, reg))
            {
                vrf.verify_bel(bel, slot_name, &[], &[]);
            }
        }
        bels::DFE_B | bels::DFE_C | bels::DFE_D | bels::DFE_E | bels::DFE_F | bels::DFE_G => {
            if !endev.edev.disabled.contains(&DisabledPart::Dfe) {
                vrf.verify_bel(bel, slot_name, &[], &[]);
            }
        }
        bels::CFG => vrf.verify_bel(
            bel,
            if endev.edev.chips[bel.die].config_kind.is_csec() {
                "CSEC_SITE"
            } else {
                "CONFIG_SITE"
            },
            &[],
            &[],
        ),
        bels::SYSMON => verify_sysmon(endev, vrf, bel),
        bels::PS => verify_ps(endev, vrf, bel),
        bels::VCU => verify_vcu(endev, vrf, bel),
        _ if slot_name.starts_with("ABUS_SWITCH") => verify_abus_switch(endev, vrf, bel),
        _ if slot_name.starts_with("VBUS_SWITCH") => vrf.verify_bel(bel, "VBUS_SWITCH", &[], &[]),

        _ if slot_name.starts_with("BUFCE_LEAF_X16") => verify_bufce_leaf_x16(vrf, bel),
        _ if slot_name.starts_with("BUFCE_LEAF") => verify_bufce_leaf(vrf, bel),
        bels::RCLK_INT_CLK => verify_rclk_int(endev, vrf, bel),

        bels::RCLK_SPLITTER => verify_rclk_splitter(endev, vrf, bel),
        bels::RCLK_HROUTE_SPLITTER => verify_rclk_hroute_splitter(endev, vrf, bel),

        _ if slot_name.starts_with("BUFCE_ROW") => verify_bufce_row(endev, vrf, bel),
        _ if slot_name.starts_with("GCLK_TEST_BUF") => verify_gclk_test_buf(vrf, bel),

        _ if slot_name.starts_with("BUFG_PS") => verify_bufg_ps(vrf, bel),
        bels::RCLK_PS => verify_rclk_ps(endev, vrf, bel),

        _ if slot_name.starts_with("HDIOB_DIFF_IN") => verify_hdiodiffin(endev, vrf, bel),
        _ if slot_name.starts_with("HDIOB") => verify_hdiob(endev, vrf, bel),
        _ if slot_name.starts_with("HDIOLOGIC") => verify_hdiologic(vrf, bel),
        _ if slot_name.starts_with("BUFGCE_HDIO") => verify_bufgce_hdio(vrf, bel),
        bels::RCLK_HDIO => verify_rclk_hdio(endev, vrf, bel),
        bels::RCLK_HDIOL => verify_rclk_hdiol(endev, vrf, bel),

        _ if slot_name.starts_with("BUFGCE_DIV") => verify_bufgce_div(vrf, bel),
        _ if slot_name.starts_with("BUFGCE") => verify_bufgce(endev, vrf, bel),
        _ if slot_name.starts_with("BUFGCTRL") => verify_bufgctrl(vrf, bel),
        bels::MMCM => verify_mmcm(endev, vrf, bel),
        bels::PLL0 | bels::PLL1 | bels::PLLXP0 | bels::PLLXP1 => verify_pll(endev, vrf, bel),
        bels::HBM_REF_CLK0 | bels::HBM_REF_CLK1 => verify_hbm_ref_clk(vrf, bel),
        bels::CMT | bels::CMTXP => verify_cmt(endev, vrf, bel),

        _ if slot_name.starts_with("BITSLICE_T") => verify_bitslice_tx(endev, vrf, bel),
        _ if slot_name.starts_with("BITSLICE_CONTROL") => verify_bitslice_control(endev, vrf, bel),
        _ if slot_name.starts_with("BITSLICE") => verify_bitslice_rx_tx(endev, vrf, bel),
        _ if slot_name.starts_with("PLL_SELECT") => verify_pll_select(endev, vrf, bel),
        _ if slot_name.starts_with("RIU_OR") => verify_riu_or(vrf, bel),
        _ if slot_name.starts_with("XIPHY_FEEDTHROUGH") => {
            verify_xiphy_feedthrough(endev, vrf, bel)
        }
        bels::XIPHY_BYTE => verify_xiphy_byte(endev, vrf, bel),
        bels::RCLK_XIPHY => verify_rclk_xiphy(endev, vrf, bel),

        bels::HPIOB_DCI0 | bels::HPIOB_DCI1 => verify_hpio_dci(endev, vrf, bel),
        _ if slot_name.starts_with("HPIOB_DIFF_IN") => verify_hpiodiffin(endev, vrf, bel),
        _ if slot_name.starts_with("HPIOB_DIFF_OUT") => verify_hpiodiffout(endev, vrf, bel),
        _ if slot_name.starts_with("HPIOB") => verify_hpiob(endev, vrf, bel),
        bels::HPIO_VREF => verify_hpio_vref(vrf, bel),
        bels::HPIO_BIAS => vrf.verify_bel(bel, "BIAS", &[], &[]),

        _ if slot_name.starts_with("HRIOB_DIFF_IN") => verify_hriodiffin(vrf, bel),
        _ if slot_name.starts_with("HRIOB_DIFF_OUT") => verify_hriodiffout(vrf, bel),
        _ if slot_name.starts_with("HRIOB") => verify_hriob(endev, vrf, bel),

        _ if slot_name.starts_with("XP5IOB") => xp5io::verify_xp5iob(vrf, bel),
        _ if slot_name.starts_with("XP5IO_VREF") => xp5io::verify_xp5io_vref(vrf, bel),
        _ if slot_name.starts_with("X5PHY_LS") => xp5io::verify_x5phy_ls(vrf, bel),
        _ if slot_name.starts_with("X5PHY_HS") => xp5io::verify_x5phy_hs(vrf, bel),
        _ if slot_name.starts_with("X5PHY_PLL_SELECT") => xp5io::verify_x5phy_pll_select(vrf, bel),
        bels::XP5PIO_CMU_ANA => xp5io::verify_xp5pio_cmu_ana(vrf, bel),
        bels::XP5PIO_CMU_DIG_TOP => xp5io::verify_xp5pio_cmu_dig(vrf, bel),
        bels::LPDDRMC => xp5io::verify_lpddrmc(vrf, bel),

        _ if slot_name.starts_with("BUFG_GT_SYNC") => verify_bufg_gt_sync(endev, vrf, bel),
        _ if slot_name.starts_with("BUFG_GT") => verify_bufg_gt(endev, vrf, bel),
        _ if slot_name.starts_with("GTH_CHANNEL")
            || slot_name.starts_with("GTY_CHANNEL")
            || slot_name.starts_with("GTF_CHANNEL") =>
        {
            verify_gt_channel(endev, vrf, bel)
        }
        bels::GTH_COMMON | bels::GTY_COMMON | bels::GTF_COMMON => verify_gt_common(endev, vrf, bel),
        bels::GTM_DUAL => verify_gtm_dual(endev, vrf, bel),
        bels::GTM_REFCLK => verify_gtm_refclk(endev, vrf, bel),
        bels::HSADC | bels::HSDAC | bels::RFADC | bels::RFDAC => {
            verify_hsadc_hsdac(endev, vrf, bel)
        }
        bels::RCLK_GT => verify_rclk_gt(endev, vrf, bel),

        _ => println!("MEOW {} {:?}", slot_name, bel.name),
    }
}

fn verify_extra(endev: &ExpandedNamedDevice, vrf: &mut Verifier) {
    for w in [
        "CLK_VDISTR_FT0",
        "CLK_VROUTE_FT0",
        "CLK_VDISTR_FT0_0",
        "CLK_VROUTE_FT0_0",
        "CLK_VDISTR_FT0_1",
        "CLK_VROUTE_FT0_1",
        "CASMBIST12IN",
        "SYNC_CLK_B_TOP0",
        "SYNC_CLK_B_TOP1",
        "SYNC_CLK_B_TOP2",
        "SYNC_CLK_B_TOP3",
        "SYNC_CLK_TOP0",
        "SYNC_CLK_TOP1",
        "SYNC_CLK_TOP2",
        "SYNC_CLK_TOP3",
        "SYNC_DIN_TOP0",
        "SYNC_DIN_TOP1",
        "SYNC_DIN_TOP2",
        "SYNC_DIN_TOP3",
        "SYNC_SR_TOP0",
        "SYNC_SR_TOP1",
        "SYNC_SR_TOP2",
        "SYNC_SR_TOP3",
        "SYNC_DOUT_BOT0",
        "SYNC_DOUT_BOT1",
        "SYNC_DOUT_BOT2",
        "SYNC_DOUT_BOT3",
        "SYNC_DOUT_TERM0",
        "SYNC_DOUT_TERM1",
        "SYNC_DOUT_TERM2",
        "SYNC_DOUT_TERM3",
        "CLOCK_DR_FT0",
        "SHIFT_DR_FT0",
        "UPDATE_DR_FT0",
        "UPDATE_DR_FT1",
        "EXTEST_FT0",
        "EXTEST_FT1",
        "INTEST_FT0",
        "INTEST_FT1",
        "MISR_JTAG_LOAD_FT0",
        "AC_MODE_FT0",
        "RESET_TAP_B_FT0",
        "FST_CFG_B_FT0",
        "FST_CFG_B_FT1",
        "GPWRDWN_B_FT0",
        "GPWRDWN_B_FT1",
        "GTS_CFG_B_FT0",
        "GTS_CFG_B_FT1",
        "GTS_USR_B_FT0",
        "GTS_USR_B_FT1",
        "POR_B_FT0",
        "POR_B_FT1",
        "CLOCK_DR_IN",
        "SHIFT_DR_IN",
        "RESET_TAP_BP",
        "EXTEST_SMPL_FT0",
        "HSWAPEN_FT0",
        "IO_TO_CTR_FT0_0",
        "IO_TO_CTR_FT0_1",
        "IO_TO_CTR_FT0_2",
        "IO_TO_CTR_FT0_3",
        "REMOTE_DIODE_FN_FT0_0",
        "REMOTE_DIODE_FN_FT0_1",
        "REMOTE_DIODE_FN_FT0_2",
        "REMOTE_DIODE_FP_FT0",
        "REMOTE_DIODE_SN_FT0_0",
        "REMOTE_DIODE_SN_FT0_1",
        "REMOTE_DIODE_SN_FT0_2",
        "REMOTE_DIODE_SP_FT0",
        "DIODE_N_OPT",
        "DIODE_P_OPT",
        "GHIGH_B_PSS_0",
        "GHIGH_B_PSS_1",
        "CFG_RESET_B_FT1_0",
        "CFG_RESET_B_FT1_1",
        "GPWRDWN_B_FT1_0",
        "GPWRDWN_B_FT1_1",
        "POR_B_FT1_0",
        "POR_B_FT1_1",
        "IDCODE19",
        "PSS_ALTO_CORE_0_IDCODE15_PIN",
        "PSS_ALTO_CORE_0_IDCODE16_PIN",
        "PSS_ALTO_CORE_0_IDCODE17_PIN",
        "PSS_ALTO_CORE_0_IDCODE18_PIN",
        "PSS_ALTO_CORE_0_IDCODE20_PIN",
        "PSS_ALTO_CORE_0_IDCODE21_PIN",
        "PSS_ALTO_CORE_0_IDCODE28_PIN",
        "PSS_ALTO_CORE_0_IDCODE29_PIN",
        "PSS_ALTO_CORE_0_PS_VERSION_0_PIN",
    ] {
        vrf.kill_stub_in_cond(w);
    }
    for w in [
        "PSS_ALTO_CORE_0_IDCODE15",
        "PSS_ALTO_CORE_0_IDCODE16",
        "PSS_ALTO_CORE_0_IDCODE17",
        "PSS_ALTO_CORE_0_IDCODE18",
        "PSS_ALTO_CORE_0_IDCODE21",
        "PSS_ALTO_CORE_0_IDCODE28",
        "PSS_ALTO_CORE_0_IDCODE29",
        "PSS_ALTO_CORE_0_IDCODE30",
        "PSS_ALTO_CORE_0_IDCODE31",
        "PSS_ALTO_CORE_0_PS_VERSION_0",
        "PSS_ALTO_CORE_0_PS_VERSION_2",
        "PSS_ALTO_CORE_0_PS_VERSION_3",
    ] {
        vrf.kill_stub_out_cond(w);
    }
    if endev.edev.kind == ChipKind::Ultrascale {
        for i in 0..104 {
            vrf.kill_stub_in_cond_tk("INT_TERM_L_IO", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_TERM_L", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_K3_TERM_L_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_VH_TERM_L_FT", &format!("GND_WIRE{i}"));
        }
        for i in 0..4 {
            vrf.kill_stub_in_cond_tk("INT_IBRK_LEFT_L_FT", &format!("GND_WIRE{i}"));
        }
        vrf.kill_stub_in_cond_tk("CFRM_L_TERM_T", "GND_WIRE0");
        vrf.kill_stub_in_cond_tk("CFRM_L_TERM_T", "GND_WIRE2");
    } else {
        for i in 0..84 {
            vrf.kill_stub_in_cond_tk("INT_INTF_LEFT_TERM_IO_FT", &format!("GND_WIRE{i}"));
        }
        for i in 0..104 {
            vrf.kill_stub_in_cond_tk("INT_INTF_RIGHT_TERM_HDIO_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("INT_INTF_RIGHT_TERM_HDIO_FT", &format!("WESTBUS_NOCONN{i}"));
        }
        for i in 0..10785 {
            vrf.kill_stub_in_cond_tk("HDIO_OUTER_TERM_R_FT", &format!("GND_WIRE{i}"));
        }
        for i in 0..3120 {
            vrf.kill_stub_in_cond_tk("HPIO_TERM_L_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_TERM_L_DA6_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_AUX_IO_TERM_L_BOT_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_AUX_IO_TERM_L_TOP_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_CFG_TERM_L_BOT_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_CFG_TERM_L_TOP_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_AUX_IO_TERM_L_BOT_DA6_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_AUX_IO_TERM_L_TOP_DA6_FT", &format!("GND_WIRE{i}"));
        }
        for i in 0..24 {
            vrf.kill_stub_in_cond(&format!("OPTION2IOB_TX_DATA_OPT{i}"));
            vrf.kill_stub_in_cond(&format!("OPTION2IOB_PERSIST_MODE_EN_OPT{i}"));
        }
        for i in 0..94 {
            vrf.kill_stub_in_cond_tk("AMS", &format!("UNCONN_INOUTS{i}"));
        }
        for i in 0..32 {
            vrf.kill_stub_in_cond_tk("CFG_CONFIG", &format!("PCAP_WDATA{i}"));
        }
        for w in [
            "PCAP_CS_B",
            "PCAP_PR",
            "PCAP_WDATA_CLK",
            "PCAP_RDWR_B",
            "PCFG_BOOT0",
            "PCFG_BOOT1",
            "PCFG_BOOT2",
            "PCFG_GSR",
            "PCFG_GTS",
            "PCFG_INIT_B",
            "PCFG_JTAG_CFG_DISABLE0",
            "PCFG_JTAG_CFG_DISABLE1",
            "PCFG_POR_CNT_4K",
            "PCFG_PROG",
            "PCFG_TCK",
            "PCFG_TDI",
            "PCFG_TMS",
            "UNCONN_INOUTS0",
            "UNCONN_INOUTS1",
            "UNCONN_INOUTS2",
            "UNCONN_INOUTS3",
            "UNCONN_INOUTS4",
            "UNCONN_INOUTS5",
            "UNCONN_INOUTS6",
            "UNCONN_INOUTS7",
            "UNCONN_INOUTS8",
        ] {
            vrf.kill_stub_in_cond_tk("CFG_CONFIG", w);
        }
        for i in 0..61 {
            if i == 30 {
                continue;
            }
            for j in 0..32 {
                vrf.kill_stub_in_cond_tk("HDIO_OUTER_TERM_R_FT", &format!("LOGIC_OUTS_{i}_{j}"));
            }
        }
        vrf.kill_stub_in_cond("OPTION2IOB_PERSIST_MODE_EN_VR1_OPT");
        vrf.kill_stub_in_cond("OPTION2IOB_PERSIST_MODE_EN_VR2_OPT");
        vrf.kill_stub_in_cond("OPTION2IOB_TX_DATA_VR1_OPT");
        vrf.kill_stub_in_cond("OPTION2IOB_TX_DATA_VR2_OPT");
        vrf.kill_stub_in_cond("CLK_FROM_EXT_LOW_SOUTH");
        vrf.kill_stub_in_cond("CLK_FROM_EXT_UPP_SOUTH");
        vrf.kill_stub_in_cond("CLK_FROM_EXT_LOW_NORTH");
        vrf.kill_stub_in_cond("CLK_FROM_EXT_UPP_NORTH");
        vrf.kill_stub_in_cond_tk("RCLK_AMS_CFGIO", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_AMS_CFGIO", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("RCLK_AMS_CFGIO", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("CFG_CONFIG", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("CFG_CONFIG", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("CFG_CONFIG", "VCC_WIRE4");
        vrf.kill_stub_in_cond_tk("CSEC_CONFIG_FT", "VCC_WIRE1603");
        vrf.kill_stub_in_cond_tk("CSEC_CONFIG_FT", "VCC_WIRE1604");
        vrf.kill_stub_in_cond_tk("RCLK_INT_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_INT_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_CLEL_L_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_CLEL_L_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_CLEL_R_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_CLEL_R_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_CLEM_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_CLEM_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_CLEM_CLKBUF_L", "VCC_WIRE4");
        vrf.kill_stub_in_cond_tk("RCLK_HDIO", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_HDIO", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_DSP_INTF_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_DSP_INTF_CLKBUF_L", "VCC_WIRE9");
        vrf.kill_stub_in_cond_tk("RCLK_RCLK_XIPHY_INNER_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_INTF_R_IBRK_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_INTF_L_IBRK_IO_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_RCLK_INTF_LEFT_IBRK_FE_L_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_INTF_L_IBRK_PCIE4_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_INTF_L_TERM_GT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_RCLK_INTF_LEFT_TERM_IO_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("HPIO_RIGHT_TERM_T", "VCC_WIRE");
        vrf.kill_stub_in_cond_tk("HPIO_RIGHT_TERM_T", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("HPIO_RIGHT_TERM_T", "GND_WIRE1");
        vrf.kill_stub_in_cond_tk("HPIO_RIGHT_TERM_T", "GND_WIRE3");
        vrf.kill_stub_in_cond_tk("HPIO_L_TERM_T", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("HPIO_L_TERM_T", "GND_WIRE2");
        vrf.kill_stub_in_cond_tk("HPIO_L_TERM_T", "GND_WIRE5");
        vrf.kill_stub_in_cond_tk("HPIO_HPIO_LEFT_TERM_T_L_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("HPIO_HPIO_LEFT_TERM_T_L_FT", "GND_WIRE1");
        vrf.kill_stub_in_cond_tk("HPIO_HPIO_LEFT_TERM_T_L_FT", "GND_WIRE3");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE0");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE1");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE2");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE3");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE4");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE5");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE6");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE7");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE8");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE9");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE10");
        vrf.kill_stub_in_cond_tk("HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", "GND_WIRE11");
        vrf.kill_stub_in_cond_tk("PCIE4_TERM_T_FT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("PCIE4_TERM_T_FT", "GND_WIRE1");
        vrf.kill_stub_in_cond_tk("PCIE4_TERM_T_FT", "GND_WIRE3");
        vrf.kill_stub_in_cond_tk("CMAC_CMAC_TERM_T_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("CMAC_CMAC_TERM_T_FT", "GND_WIRE1");
        vrf.kill_stub_in_cond_tk("CMAC_CMAC_TERM_T_FT", "GND_WIRE3");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "VCC_WIRE4");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "VCC_WIRE6");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "GND_WIRE0");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "GND_WIRE2");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "GND_WIRE5");
        vrf.kill_stub_in_cond_tk("CMAC", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("CMAC", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("CMAC", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("CMAC", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("PCIE4_PCIE4_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("PCIE4_PCIE4_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("PCIE4_PCIE4_FT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("PCIE4_PCIE4_FT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("PCIE4C_PCIE4C_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("PCIE4C_PCIE4C_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("PCIE4C_PCIE4C_FT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("PCIE4C_PCIE4C_FT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("ILKN_ILKN_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("ILKN_ILKN_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("ILKN_ILKN_FT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("ILKN_ILKN_FT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("FE_FE_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEA_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEA_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEA_FT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEA_FT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEB_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEB_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEB_FT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEB_FT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEC_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEC_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILED_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILED_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEE_FT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEE_FT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEF_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEF_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEG_FT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEG_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEG_FT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("DFE_DFE_TILEG_FT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("HDIO_TOP_RIGHT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("HDIO_TOP_RIGHT", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("HDIO_TOP_RIGHT", "VCC_WIRE5");
        vrf.kill_stub_in_cond_tk("HDIO_TOP_RIGHT", "VCC_WIRE7");
        vrf.kill_stub_in_cond_tk("HDIO_TOP_RIGHT", "GND_WIRE1");
        vrf.kill_stub_in_cond_tk("HDIO_TOP_RIGHT", "GND_WIRE3");
        vrf.kill_stub_in_cond_tk("HDIO_TOP_RIGHT", "GND_WIRE6");
        vrf.kill_stub_in_cond_tk("VCU_VCU_FT", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("VCU_VCU_FT", "VCC_WIRE4");
        vrf.kill_stub_in_cond_tk("VCU_VCU_FT", "GHIGH_B_PSS");
        vrf.kill_stub_in_cond_tk("VCU_VCU_FT", "PL_VCU_GLOBAL_CFG_RESET_B");
        vrf.kill_stub_in_cond_tk("BRAM", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("BRAM", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("BRAM", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("BRAM", "VCC_WIRE3");
        if let Some((_, tk)) = vrf.rd.tile_kinds.get("PSS_ALTO") {
            for &crd in &tk.tiles {
                for (wt, wf) in [
                    ("PSS_ALTO_CORE_0_IDCODE15_PIN", "PSS_ALTO_CORE_0_IDCODE15"),
                    ("PSS_ALTO_CORE_0_IDCODE16_PIN", "PSS_ALTO_CORE_0_IDCODE16"),
                    ("PSS_ALTO_CORE_0_IDCODE17_PIN", "PSS_ALTO_CORE_0_IDCODE17"),
                    ("PSS_ALTO_CORE_0_IDCODE18_PIN", "PSS_ALTO_CORE_0_IDCODE18"),
                    ("PSS_ALTO_CORE_0_IDCODE20_PIN", "PSS_ALTO_CORE_0_IDCODE20"),
                    ("PSS_ALTO_CORE_0_IDCODE21_PIN", "PSS_ALTO_CORE_0_IDCODE21"),
                    ("PSS_ALTO_CORE_0_IDCODE28_PIN", "PSS_ALTO_CORE_0_IDCODE28"),
                    ("PSS_ALTO_CORE_0_IDCODE29_PIN", "PSS_ALTO_CORE_0_IDCODE29"),
                    (
                        "PSS_ALTO_CORE_0_PS_VERSION_0_PIN",
                        "PSS_ALTO_CORE_0_PS_VERSION_0",
                    ),
                ] {
                    if vrf.rd.wires.contains(wt) && vrf.rd.wires.contains(wf) {
                        vrf.claim_pip(crd, wt, wf);
                    }
                }
            }
        }
    }
}

fn verify_pre(vrf: &mut Verifier) {
    if let Some((tcid, tcls)) = vrf.db.tile_classes.get("XP5IO") {
        let BelInfo::Bel(bel) = &tcls.bels[bels::LPDDRMC] else {
            unreachable!()
        };
        for &tcrd in &vrf.grid.tile_index[tcid] {
            for (opin, ipin) in [
                ("CFG2IOB_PUDC_B_O", "CFG2IOB_PUDC_B"),
                ("IJTAG_RESET_TAP_O", "IJTAG_RESET_TAP"),
                ("CAPTURE_DR_O", "CAPTURE_DR"),
                ("SELECT_DR_O", "SELECT_DR"),
            ] {
                let wo = *bel.pins[opin].wires.iter().next().unwrap();
                let wo = vrf.grid.tile_wire(tcrd, wo);
                let wi = *bel.pins[ipin].wires.iter().next().unwrap();
                let wi = vrf.grid.tile_wire(tcrd, wi);
                vrf.alias_intf_int(wo, wi);
            }
        }
    }
}

pub fn verify_device(endev: &ExpandedNamedDevice, rd: &Part) {
    verify(
        rd,
        &endev.ngrid,
        verify_pre,
        |vrf, bel| verify_bel(endev, vrf, bel),
        |vrf| verify_extra(endev, vrf),
    );
}
