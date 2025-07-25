use prjcombine_ecp::{
    bels,
    chip::{ChipKind, RowKind},
    tslots,
};
use prjcombine_interconnect::{
    db::CellSlotId,
    grid::{CellCoord, DieId},
};
use unnamed_entity::EntityId;

use crate::ChipContext;

mod ecp;
mod ecp2;
mod ecp3;
mod machxo;

impl ChipContext<'_> {
    fn process_hsdclk_splitter(&mut self) {
        let hsdclk = [
            self.intdb.get_wire("HSDCLK0"),
            self.intdb.get_wire("HSDCLK4"),
        ];
        for (tcrd, tile) in self.edev.egrid.tiles() {
            if tcrd.slot != tslots::HSDCLK_SPLITTER {
                continue;
            }
            let bcrd = tcrd.bel(bels::HSDCLK_SPLITTER);
            self.name_bel_null(bcrd);
            let cell = tcrd.cell.delta(-1, 0);
            for i in 0..8 {
                let wire_w = tile.cells[CellSlotId::from_idx(i % 4)].wire(hsdclk[i / 4]);
                let wire_e = tile.cells[CellSlotId::from_idx(i % 4 + 4)].wire(hsdclk[i / 4]);
                let wire_l2r = self.rc_wire(cell, &format!("HSSX0{i}00_L2R"));
                let wire_r2l = self.rc_wire(cell, &format!("HSSX0{i}00_R2L"));
                self.add_bel_wire(bcrd, format!("HSDCLK{i}_L2R"), wire_l2r);
                self.add_bel_wire(bcrd, format!("HSDCLK{i}_R2L"), wire_r2l);
                self.claim_pip_int_out(wire_w, wire_r2l);
                self.claim_pip_int_in(wire_r2l, wire_e);
                self.claim_pip_int_out(wire_e, wire_l2r);
                self.claim_pip_int_in(wire_l2r, wire_w);
            }
        }
    }

    fn process_hsdclk_vcc(&mut self) {
        for (row, rd) in &self.chip.rows {
            if rd.kind != RowKind::Io && !rd.sclk_break {
                continue;
            }
            for (col, cd) in &self.chip.columns {
                if col != self.chip.col_w() && !cd.sdclk_break {
                    continue;
                }
                let mut cell = CellCoord::new(DieId::from_idx(0), col, row);
                if !self.edev.egrid.has_bel(cell.bel(bels::INT)) {
                    cell.row += 9;
                }
                for i in 0..8 {
                    let wire = cell.wire(self.intdb.get_wire(&format!("HSDCLK{i}")));
                    let vcc = cell.wire(self.intdb.get_wire("TIE1"));
                    let vcc = self.naming.interconnect[&vcc];
                    self.claim_pip_int_out(wire, vcc);
                }
            }
        }
    }

    pub fn process_clk(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp => self.process_clk_ecp(),
            ChipKind::MachXo => self.process_clk_machxo(),
            ChipKind::Ecp2 | ChipKind::Ecp2M => {
                self.process_hsdclk_splitter();
                self.process_clk_ecp2();
            }
            ChipKind::Xp2 => {
                self.process_hsdclk_splitter();
                self.process_clk_ecp2();
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.process_hsdclk_splitter();
                self.process_hsdclk_vcc();
                let roots = self.process_hsdclk_root_ecp3();
                self.process_clk_ecp3(roots);
                self.process_pclk_ecp3();
            }
        }
    }

    pub fn process_clk_zones(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp | ChipKind::Xp | ChipKind::MachXo => (),
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => self.process_clk_zones_ecp2(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => (),
        }
    }
}
