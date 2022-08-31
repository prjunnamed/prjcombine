use prjcombine_entity::EntityId;
use prjcombine_int::db::NodeRawTileId;
use prjcombine_int::grid::{ColId, DieId, RowId};
use prjcombine_rawdump::Coord;
use prjcombine_rdverify::{BelContext, SitePinDir, Verifier};
use prjcombine_virtex2::{Edge, ExpandedDevice, GridKind, IoDiffKind};

fn verify_pci_ce(
    edev: &ExpandedDevice<'_>,
    vrf: &mut Verifier,
    die: DieId,
    col: ColId,
    row: RowId,
    crd: Coord,
    wire: &str,
) {
    if col == edev.grid.col_left() || col == edev.grid.col_right() {
        if row < edev.grid.row_mid() {
            for &(srow, _, _) in &edev.grid.rows_hclk {
                if srow > edev.grid.row_mid() {
                    break;
                }
                if row < srow {
                    let obel = vrf.find_bel(die, (col, srow), "PCI_CE_S").unwrap();
                    vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
                    return;
                }
            }
        } else {
            for &(srow, _, _) in edev.grid.rows_hclk.iter().rev() {
                if srow <= edev.grid.row_mid() {
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
            .find_bel(die, (col, edev.grid.row_mid() - 1), "PCILOGICSE")
            .unwrap();
        let pip = &obel.naming.pins["PCI_CE"].pips[0];
        vrf.verify_node(&[(obel.crds[pip.tile], &pip.wire_to), (crd, wire)]);
    } else {
        if edev.grid.kind == GridKind::Spartan3A {
            if let Some((col_l, col_r)) = edev.grid.cols_clkv {
                if col >= col_l && col < col_r {
                    let (scol, kind) = if col < edev.grid.col_clk {
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
        let scol = if col < edev.grid.col_clk {
            edev.grid.col_left()
        } else {
            edev.grid.col_right()
        };
        let obel = vrf.find_bel(die, (scol, row), "PCI_CE_CNR").unwrap();
        vrf.verify_node(&[obel.fwire("O"), (crd, wire)]);
    }
}

pub fn verify_ioi(edev: &ExpandedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let io = edev.get_io((bel.col, bel.row), bel.bid);
    let tn = &bel.node.names[NodeRawTileId::from_idx(0)];
    let is_ipad = tn.contains("IBUFS") || (tn.contains("IOIB") && bel.bid.to_idx() == 2);
    let kind = if matches!(edev.grid.kind, GridKind::Spartan3A | GridKind::Spartan3ADsp) {
        let is_tb = matches!(io.bank, 0 | 2);
        match (io.diff, is_ipad) {
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
        match (io.diff, is_ipad) {
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
        edev.grid.kind,
        GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
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
    if edev.grid.kind == GridKind::Spartan3ADsp {
        pins.extend([("OAUX", SitePinDir::In), ("TAUX", SitePinDir::In)]);
    }
    vrf.verify_bel(bel, kind, &pins, &[]);
    // diff pairing
    if !edev.grid.kind.is_virtex2() || io.diff != IoDiffKind::None {
        for pin in ["PADOUT", "DIFFI_IN", "DIFFO_IN", "DIFFO_OUT"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
        match io.diff {
            IoDiffKind::P(obid) => {
                let obel = vrf.get_bel(bel.die, bel.node, obid);
                vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
            }
            IoDiffKind::N(obid) => {
                let obel = vrf.get_bel(bel.die, bel.node, obid);
                vrf.claim_pip(bel.crd(), bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
                vrf.claim_pip(bel.crd(), bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
            }
            IoDiffKind::None => (),
        }
    }
    if matches!(
        edev.grid.kind,
        GridKind::Spartan3E | GridKind::Spartan3A | GridKind::Spartan3ADsp
    ) {
        for pin in [
            "ODDRIN1", "ODDRIN2", "ODDROUT1", "ODDROUT2", "IDDRIN1", "IDDRIN2", "PCI_CE", "PCI_RDY",
        ] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
        // ODDR, IDDR
        if let IoDiffKind::P(obid) | IoDiffKind::N(obid) = io.diff {
            let obel = vrf.get_bel(bel.die, bel.node, obid);
            vrf.claim_pip(bel.crd(), bel.wire("ODDRIN1"), obel.wire("ODDROUT2"));
            vrf.claim_pip(bel.crd(), bel.wire("ODDRIN2"), obel.wire("ODDROUT1"));
            vrf.claim_pip(bel.crd(), bel.wire("IDDRIN1"), obel.wire("IQ1"));
            vrf.claim_pip(bel.crd(), bel.wire("IDDRIN2"), obel.wire("IQ2"));
        }
        vrf.claim_pip(bel.crd(), bel.wire("PCI_CE"), bel.wire_far("PCI_CE"));
        verify_pci_ce(
            edev,
            vrf,
            bel.die,
            bel.col,
            bel.row,
            bel.crd(),
            bel.wire_far("PCI_CE"),
        );
    }
    if edev.grid.kind == GridKind::Spartan3ADsp {
        for pin in ["OAUX", "TAUX"] {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
}

pub fn verify_pcilogicse(edev: &ExpandedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    let edge = if bel.col == edev.grid.col_left() {
        Edge::Left
    } else if bel.col == edev.grid.col_right() {
        Edge::Right
    } else {
        unreachable!()
    };
    let pci_rdy = edev.grid.get_pci_io(edge);
    for (pin, (crd, obid)) in ["IRDY", "TRDY"].into_iter().zip(pci_rdy.into_iter()) {
        vrf.claim_node(&[bel.fwire(pin)]);
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
        let onode = edev.get_io_node(crd).unwrap();
        let obel = vrf.get_bel(bel.die, onode, obid);
        vrf.claim_node(&[bel.fwire_far(pin), obel.fwire("PCI_RDY_IN")]);
        vrf.claim_pip(obel.crd(), obel.wire("PCI_RDY_IN"), obel.wire("PCI_RDY"));
    }
    let pip = &bel.naming.pins["PCI_CE"].pips[0];
    vrf.claim_node(&[bel.fwire("PCI_CE"), (bel.crds[pip.tile], &pip.wire_from)]);
    vrf.claim_pip(bel.crds[pip.tile], &pip.wire_to, &pip.wire_from);
    vrf.claim_node(&[(bel.crds[pip.tile], &pip.wire_to)]);
}

pub fn verify_pci_ce_n(edev: &ExpandedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        edev,
        vrf,
        bel.die,
        bel.col,
        bel.row - 1,
        bel.crd(),
        bel.wire("I"),
    );
}

pub fn verify_pci_ce_s(edev: &ExpandedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        edev,
        vrf,
        bel.die,
        bel.col,
        bel.row,
        bel.crd(),
        bel.wire("I"),
    );
}

pub fn verify_pci_ce_e(edev: &ExpandedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        edev,
        vrf,
        bel.die,
        bel.col - 1,
        bel.row,
        bel.crd(),
        bel.wire("I"),
    );
}

pub fn verify_pci_ce_w(edev: &ExpandedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        edev,
        vrf,
        bel.die,
        bel.col,
        bel.row,
        bel.crd(),
        bel.wire("I"),
    );
}

pub fn verify_pci_ce_cnr(edev: &ExpandedDevice<'_>, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("O")]);
    vrf.claim_pip(bel.crd(), bel.wire("O"), bel.wire("I"));
    verify_pci_ce(
        edev,
        vrf,
        bel.die,
        bel.col,
        bel.row,
        bel.crd(),
        bel.wire("I"),
    );
}
