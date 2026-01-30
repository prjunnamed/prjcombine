use prjcombine_interconnect::{
    db::TileWireCoord,
    dir::{Dir, DirV},
    grid::BelCoord,
};
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rdverify::{RawWireCoord, Verifier};
use prjcombine_virtex2::{
    chip::ChipKind,
    defs::{bcls, bslots, spartan3::wires as wires_s3},
};

pub fn verify_bufgmux(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf.verify_bel(bcrd);
    if endev.edev.chip.kind == ChipKind::FpgaCore {
        bel = bel.kind("BUFG").rename_in(bcls::BUFGMUX::I0, "I");
    } else {
        let edge = if bcrd.row == endev.chip.row_s() {
            Dir::S
        } else if bcrd.row == endev.chip.row_n() {
            Dir::N
        } else if bcrd.col == endev.chip.col_w() {
            Dir::W
        } else if bcrd.col == endev.chip.col_e() {
            Dir::E
        } else {
            unreachable!()
        };
        if matches!(edge, Dir::H(_)) {
            let tcrd = endev.edev.get_tile_by_bel(bcrd);
            let ntile = &endev.ngrid.tiles[&tcrd];
            let naming = &endev.ngrid.db.tile_class_namings[ntile.naming];
            bel.claim_pip(
                bel.wire("S"),
                RawWireCoord {
                    crd: bel.crd(),
                    wire: &naming.wires[&TileWireCoord::new_idx(4, wires_s3::PULLUP)].name,
                },
            );
        }
    }
    bel.commit();
}

pub fn verify_globalsig_bufg(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::GLOBALSIG_BUFG.index_of(bcrd.slot).unwrap();
    let mut bel = vrf.verify_bel(bcrd).kind("GLOBALSIG");
    if endev.edev.chip.kind == ChipKind::Virtex2P {
        bel.claim_net(&[bel.wire("BREFCLK_O")]);
        bel.claim_pip(bel.wire("BREFCLK_O"), bel.wire("BREFCLK_I"));
        let edge = if bcrd.row == endev.chip.row_s() {
            DirV::S
        } else {
            DirV::N
        };
        let io = endev.chip.get_brefclk_io(edge)[idx];
        let io = endev.chip.get_io_loc(io);
        bel.verify_net(&[bel.wire("BREFCLK_I"), bel.bel_wire(io, "CLKPAD")]);
    }
    bel.commit();
}
