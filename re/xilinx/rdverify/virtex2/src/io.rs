use prjcombine_interconnect::db::Dir;
use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use prjcombine_re_xilinx_naming::db::NodeRawTileId;
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rawdump::Coord;
use prjcombine_re_xilinx_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_virtex2::chip::{ChipKind, IoDiffKind};
use unnamed_entity::EntityId;

use crate::get_bel_iob;

fn verify_pci_ce(
    endev: &ExpandedNamedDevice,
    vrf: &mut Verifier,
    die: DieId,
    col: ColId,
    row: RowId,
    crd: Coord,
    wire: &str,
) {
    if col == endev.chip.col_left() || col == endev.chip.col_right() {
        if row < endev.chip.row_mid() {
            for &(srow, _, _) in &endev.chip.rows_hclk {
                if srow > endev.chip.row_mid() {
                    break;
                }
                if row < srow {
                    let obel = vrf.find_bel(die, (col, srow), "PCI_CE_S").unwrap();
                    vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
                    return;
                }
            }
        } else {
            for &(srow, _, _) in endev.chip.rows_hclk.iter().rev() {
                if srow <= endev.chip.row_mid() {
                    break;
                }
                if row >= srow {
                    let obel = vrf.find_bel(die, (col, srow), "PCI_CE_N").unwrap();
                    vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
                    return;
                }
            }
        }
        let obel = vrf
            .find_bel(die, (col, endev.chip.row_mid()), "PCILOGICSE")
            .unwrap();
        let pip = &obel.naming.pins["PCI_CE"].pips[0];
        vrf.verify_node(&[(obel.crds[pip.tile], &pip.wire_to), (crd, wire)]);
    } else {
        if endev.chip.kind == ChipKind::Spartan3A {
            if let Some((col_l, col_r)) = endev.chip.cols_clkv {
                if col >= col_l && col < col_r {
                    let (scol, kind) = if col < endev.chip.col_clk {
                        (col_l, "PCI_CE_E")
                    } else {
                        (col_r, "PCI_CE_W")
                    };
                    let obel = vrf.find_bel(die, (scol, row), kind).unwrap();
                    vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
                    return;
                }
            }
        }
        let scol = if col < endev.chip.col_clk {
            endev.chip.col_left()
        } else {
            endev.chip.col_right()
        };
        let obel = vrf.find_bel(die, (scol, row), "PCI_CE_CNR").unwrap();
        vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
    }
}

pub fn verify_ioi(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let io = endev.chip.get_io_crd(bel.col, bel.row, bel.bid);
    let io_info = endev.chip.get_io_info(io);
    let tn = &bel.nnode.names[NodeRawTileId::from_idx(0)];
    let is_ipad = tn.contains("IBUFS") || (tn.contains("IOIB") && bel.bid.to_idx() == 2);
    let kind = if matches!(
        endev.chip.kind,
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp
    ) {
        let is_tb = matches!(io_info.bank, 0 | 2);
        match (io_info.diff, is_ipad) {
            (IoDiffKind::P(_), false) => {
                if is_tb {
                    "DIFFMTB"
                } else {
                    "DIFFMLR"
                }
            }
            (IoDiffKind::P(_), true) => "DIFFMI_NDT",
            (IoDiffKind::N(_), false) => {
                if is_tb {
                    "DIFFSTB"
                } else {
                    "DIFFSLR"
                }
            }
            (IoDiffKind::N(_), true) => "DIFFSI_NDT",
            (IoDiffKind::None, false) => "IOB",
            (IoDiffKind::None, true) => "IBUF",
        }
    } else {
        match (io_info.diff, is_ipad) {
            (IoDiffKind::P(_), false) => "DIFFM",
            (IoDiffKind::P(_), true) => "DIFFMI",
            (IoDiffKind::N(_), false) => "DIFFS",
            (IoDiffKind::N(_), true) => "DIFFSI",
            (IoDiffKind::None, false) => "IOB",
            (IoDiffKind::None, true) => "IBUF",
        }
    };
    let mut pins = vec![
        ("PADOUT", SitePinDir::Out),
        ("DIFFI_IN", SitePinDir::In),
        ("DIFFO_OUT", SitePinDir::Out),
        ("DIFFO_IN", SitePinDir::In),
    ];
    if matches!(
        endev.chip.kind,
        ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp
    ) {
        pins.extend([
            ("PCI_RDY", SitePinDir::Out),
            ("PCI_CE", SitePinDir::In),
            ("ODDROUT1", SitePinDir::Out),
            ("ODDROUT2", SitePinDir::Out),
            ("ODDRIN1", SitePinDir::In),
            ("ODDRIN2", SitePinDir::In),
            ("IDDRIN1", SitePinDir::In),
            ("IDDRIN2", SitePinDir::In),
        ]);
    }
    if endev.chip.kind == ChipKind::Spartan3ADsp {
        pins.extend([("OAUX", SitePinDir::In), ("TAUX", SitePinDir::In)]);
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
    // diff pairing
    if !endev.chip.kind.is_virtex2() || io_info.diff != IoDiffKind::None {
        for pin in ["PADOUT", "DIFFI_IN", "DIFFO_IN", "DIFFO_OUT"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
        match io_info.diff {
            IoDiffKind::P(oiob) => {
                let obel = get_bel_iob(endev, vrf, io.with_iob(oiob));
                vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
            }
            IoDiffKind::N(oiob) => {
                let obel = get_bel_iob(endev, vrf, io.with_iob(oiob));
                vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
                vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
            }
            IoDiffKind::None => (),
        }
    }
    if matches!(
        endev.chip.kind,
        ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp
    ) {
        for pin in [
            "ODDRIN1", "ODDRIN2", "ODDROUT1", "ODDROUT2", "IDDRIN1", "IDDRIN2", "PCI_CE", "PCI_RDY",
        ] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
        // ODDR, IDDR
        if let IoDiffKind::P(oiob) | IoDiffKind::N(oiob) = io_info.diff {
            let obel = get_bel_iob(endev, vrf, io.with_iob(oiob));
            vrf.claim_pip(bel.crd(), bel.wire("ODDRIN1"), obel.wire("ODDROUT2"));
            vrf.claim_pip(bel.crd(), bel.wire("ODDRIN2"), obel.wire("ODDROUT1"));
            vrf.claim_pip(bel.crd(), bel.wire("IDDRIN1"), obel.wire("IQ1"));
            vrf.claim_pip(bel.crd(), bel.wire("IDDRIN2"), obel.wire("IQ2"));
        }
        vrf.claim_pip(bel.crd(), bel.wire("PCI_CE"), bel.wire_far("PCI_CE"));
        verify_pci_ce(
            endev,
            vrf,
            bel.die,
            bel.col,
            bel.row,
            bel.crd(),
            bel.wire_far("PCI_CE"),
        );
    }
    if endev.chip.kind == ChipKind::Spartan3ADsp {
        for pin in ["OAUX", "TAUX"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
}

pub fn verify_pcilogicse(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        "PCILOGICSE",
        &[
            ("IRDY", SitePinDir::In),
            ("TRDY", SitePinDir::In),
            ("PCI_CE", SitePinDir::Out),
        ],
        &[],
    );
    let edge = if bel.col == endev.chip.col_left() {
        Dir::W
    } else if bel.col == endev.chip.col_right() {
        Dir::E
    } else {
        unreachable!()
    };
    let pci_rdy = endev.chip.get_pci_io(edge);
    for (pin, crd) in ["IRDY", "TRDY"].into_iter().zip(pci_rdy) {
        vrf.claim_node(&[bel.fwire(pin)]);
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
        let obel = get_bel_iob(endev, vrf, crd);
        vrf.claim_node(&[bel.fwire_far(pin), obel.fwire("PCI_RDY_IN")]);
        vrf.claim_pip(obel.crd(), obel.wire("PCI_RDY_IN"), obel.wire("PCI_RDY"));
    }
    let pip = &bel.naming.pins["PCI_CE"].pips[0];
    vrf.claim_node(&[bel.fwire("PCI_CE"), (bel.crds[pip.tile], &pip.wire_from)]);
    vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
    vrf.claim_node(&[(bel.crds[pip.tile], &pip.wire_to)]);
}

pub fn verify_pci_ce_n(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        endev,
        vrf,
        bel.die,
        bel.col,
        bel.row - 1,
        bel.crd(),
        bel.wire("I"),
    );
}

pub fn verify_pci_ce_s(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        endev,
        vrf,
        bel.die,
        bel.col,
        bel.row,
        bel.crd(),
        bel.wire("I"),
    );
}

pub fn verify_pci_ce_e(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        endev,
        vrf,
        bel.die,
        bel.col - 1,
        bel.row,
        bel.crd(),
        bel.wire("I"),
    );
}

pub fn verify_pci_ce_w(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        endev,
        vrf,
        bel.die,
        bel.col,
        bel.row,
        bel.crd(),
        bel.wire("I"),
    );
}

pub fn verify_pci_ce_cnr(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        endev,
        vrf,
        bel.die,
        bel.col,
        bel.row,
        bel.crd(),
        bel.wire("I"),
    );
}
