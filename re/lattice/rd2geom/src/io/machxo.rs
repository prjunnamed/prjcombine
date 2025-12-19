use prjcombine_ecp::{bels, chip::IoGroupKind};
use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::LegacyBel,
    dir::Dir,
    grid::{BelCoord, CellCoord, DieId},
};

use crate::{ChipContext, chip::ChipExt};

impl ChipContext<'_> {
    fn process_single_io_machxo(&mut self, bcrd: BelCoord) {
        let idx = bels::IO.iter().position(|&slot| slot == bcrd.slot).unwrap();
        let cell = bcrd.cell;
        let abcd = ['A', 'B', 'C', 'D', 'E', 'F'][idx];
        let io = self.chip.get_io_crd(bcrd);
        let (r, c) = self.rc(cell);
        let name = match io.edge() {
            Dir::W => format!("PL{r}{abcd}"),
            Dir::E => format!("PR{r}{abcd}"),
            Dir::S => format!("PB{c}{abcd}"),
            Dir::N => format!("PT{c}{abcd}"),
        };
        self.name_bel(bcrd, [name]);

        let mut bel = LegacyBel::default();

        let ddtd0 = self.rc_io_wire(cell, &format!("JDDTD0{abcd}"));
        let ddtd1 = self.rc_io_wire(cell, &format!("JDDTD1{abcd}"));
        self.add_bel_wire(bcrd, "DDTD0", ddtd0);
        self.add_bel_wire(bcrd, "DDTD1", ddtd1);
        bel.pins
            .insert("DDTD0".into(), self.xlat_int_wire(bcrd, ddtd0));
        bel.pins
            .insert("DDTD1".into(), self.xlat_int_wire(bcrd, ddtd1));

        let dd2 = self.rc_io_wire(cell, &format!("JDD2{abcd}"));
        self.add_bel_wire(bcrd, "DD2", dd2);

        let (plc_cell, plc_idx) = self.chip.io_direct_plc[&io];
        let slice = plc_cell.bel(bels::SLICE[usize::from(plc_idx / 2)]);
        let plcf = self.naming.bel_wire(slice, &format!("F{}_IO", plc_idx % 2));
        let plcq = self.naming.bel_wire(slice, &format!("Q{}_IO", plc_idx % 2));

        let dd = self.rc_wire(self.chip.xlat_rc_wire(plcf), &format!("JDD{plc_idx}"));
        self.add_bel_wire(bcrd, "DD", dd);
        self.claim_pip(dd, plcf);
        self.claim_pip(dd, plcq);
        self.claim_pip(dd2, dd);

        let paddo = self.rc_io_wire(cell, &format!("JPADDO{abcd}"));
        let paddt = self.rc_io_wire(cell, &format!("JPADDT{abcd}"));
        self.add_bel_wire(bcrd, "PADDO", paddo);
        self.add_bel_wire(bcrd, "PADDT", paddt);
        self.claim_pip(paddo, dd2);
        self.claim_pip(paddo, ddtd0);
        self.claim_pip(paddo, ddtd1);
        self.claim_pip(paddt, ddtd0);
        self.claim_pip(paddt, ddtd1);

        let paddo_pio = self.rc_io_wire(cell, &format!("JPADDO{abcd}_PIO"));
        let paddt_pio = self.rc_io_wire(cell, &format!("JPADDT{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDO_PIO", paddo_pio);
        self.add_bel_wire(bcrd, "PADDT_PIO", paddt_pio);
        self.claim_pip(paddo_pio, paddo);
        self.claim_pip(paddt_pio, paddt);

        let paddi_pio = self.rc_io_wire(cell, &format!("JPADDI{abcd}_PIO"));
        self.add_bel_wire(bcrd, "PADDI_PIO", paddi_pio);
        bel.pins
            .insert("PADDI".into(), self.xlat_int_wire(bcrd, paddi_pio));

        self.insert_bel(bcrd, bel);
    }

    fn process_pictest(&mut self, cell: CellCoord) {
        let (r, c) = self.rc(cell);
        let lr = if c == 1 { 'L' } else { 'R' };
        let bcrd2 = cell.bel(bels::IO2);
        let bcrd3 = cell.bel(bels::IO3);
        self.name_bel(bcrd2, [format!("{lr}PICTEST{r}")]);
        self.name_bel_null(bcrd3);

        let mut bel = LegacyBel::default();
        let ddtd0 = self.rc_io_wire(cell, "JC0_PICTEST");
        let ddtd1 = self.rc_io_wire(cell, "JC1_PICTEST");
        self.add_bel_wire(bcrd2, "DDTD0", ddtd0);
        self.add_bel_wire(bcrd2, "DDTD1", ddtd1);
        bel.pins
            .insert("DDTD0".into(), self.xlat_int_wire(bcrd2, ddtd0));
        bel.pins
            .insert("DDTD1".into(), self.xlat_int_wire(bcrd2, ddtd1));
        let paddi_pio = self.rc_io_wire(cell, "JQ0_PICTEST");
        self.add_bel_wire(bcrd2, "PADDI_PIO", paddi_pio);
        bel.pins
            .insert("PADDI".into(), self.xlat_int_wire(bcrd2, paddi_pio));
        self.insert_bel(bcrd2, bel);

        let mut bel = LegacyBel::default();
        let ddtd0 = self.rc_io_wire(cell, "JD0_PICTEST");
        let ddtd1 = self.rc_io_wire(cell, "JD1_PICTEST");
        self.add_bel_wire(bcrd3, "DDTD0", ddtd0);
        self.add_bel_wire(bcrd3, "DDTD1", ddtd1);
        bel.pins
            .insert("DDTD0".into(), self.xlat_int_wire(bcrd3, ddtd0));
        bel.pins
            .insert("DDTD1".into(), self.xlat_int_wire(bcrd3, ddtd1));
        let paddi_pio = self.rc_io_wire(cell, "JQ1_PICTEST");
        self.add_bel_wire(bcrd3, "PADDI_PIO", paddi_pio);
        bel.pins
            .insert("PADDI".into(), self.xlat_int_wire(bcrd3, paddi_pio));
        self.insert_bel(bcrd3, bel);
    }

    pub(super) fn process_io_machxo(&mut self) {
        for (row, rd) in &self.chip.rows {
            let num_io = match rd.io_w {
                IoGroupKind::None => 0,
                IoGroupKind::Double => 2,
                IoGroupKind::Quad | IoGroupKind::QuadReverse => 4,
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_w(), row);
            for i in 0..num_io {
                self.process_single_io_machxo(cell.bel(bels::IO[i]));
            }
            if num_io == 2 {
                self.process_pictest(cell);
            }
            let num_io = match rd.io_e {
                IoGroupKind::None => 0,
                IoGroupKind::Double => 2,
                IoGroupKind::Quad | IoGroupKind::QuadReverse => 4,
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), row);
            for i in 0..num_io {
                self.process_single_io_machxo(cell.bel(bels::IO[i]));
            }
            if num_io == 2 {
                self.process_pictest(cell);
            }
        }
        for (col, cd) in &self.chip.columns {
            let num_io = match cd.io_s {
                IoGroupKind::None => 0,
                IoGroupKind::Quad | IoGroupKind::QuadReverse => 4,
                IoGroupKind::Hex | IoGroupKind::HexReverse => 6,
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), col, self.chip.row_s());
            for i in 0..num_io {
                self.process_single_io_machxo(cell.bel(bels::IO[i]));
            }
            let num_io = match cd.io_n {
                IoGroupKind::None => 0,
                IoGroupKind::Quad | IoGroupKind::QuadReverse => 4,
                IoGroupKind::Hex | IoGroupKind::HexReverse => 6,
                _ => unreachable!(),
            };
            let cell = CellCoord::new(DieId::from_idx(0), col, self.chip.row_n());
            for i in 0..num_io {
                self.process_single_io_machxo(cell.bel(bels::IO[i]));
            }
        }
    }
}
