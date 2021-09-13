use super::{
    GeomDb, Grid, Port, PortClass, PortConn, TieState, Tile, TileClass, WireClass, WireConn,
};

pub trait GeomDbBuilder {
    fn make_horiz_bus(&mut self, name: &str) -> usize;
    fn make_vert_bus(&mut self, name: &str) -> usize;
    fn make_port_slot(&mut self, name: &str) -> usize;
    fn make_port_term(&mut self, name: &str, slot: usize) -> usize;
    fn make_port_pair(&mut self, names: (&str, &str), slots: (usize, usize)) -> (usize, usize);
    fn make_tile_slot(&mut self, name: &str) -> usize;
    fn make_tile(&mut self, name: &str, cells: &[(usize, usize, usize)]) -> usize;
    fn make_tile_single(&mut self, name: &str, slot: usize) -> usize;
    fn make_wire(&mut self, name: &str, cls: &str, has_multicell_drive: bool) -> usize;
    fn make_hbus_wire(
        &mut self,
        name: &str,
        cls: &str,
        bus: usize,
        has_multicell_drive: bool,
    ) -> usize;
    fn make_vbus_wire(
        &mut self,
        name: &str,
        cls: &str,
        bus: usize,
        has_multicell_drive: bool,
    ) -> usize;
    fn make_tie_wire(&mut self, name: &str, cls: &str, state: TieState) -> usize;
    fn make_simple_pconn(
        &mut self,
        wire_down: usize,
        wire_up: usize,
        pcls_down: usize,
        pcls_up: usize,
    ) -> usize;
}

impl GeomDbBuilder for GeomDb {
    fn make_horiz_bus(&mut self, name: &str) -> usize {
        self.horiz_bus.push(name.to_string())
    }
    fn make_vert_bus(&mut self, name: &str) -> usize {
        self.vert_bus.push(name.to_string())
    }
    fn make_port_slot(&mut self, name: &str) -> usize {
        self.port_slots.push(name.to_string())
    }
    fn make_port_term(&mut self, name: &str, slot: usize) -> usize {
        let res = self.ports.push(PortClass {
            name: name.to_string(),
            slot,
            opposite: 0,
            conns: Vec::new(),
        });
        self.ports[res].opposite = res;
        res
    }
    fn make_port_pair(&mut self, names: (&str, &str), slots: (usize, usize)) -> (usize, usize) {
        let res_a = self.ports.push(PortClass {
            name: names.0.to_string(),
            slot: slots.0,
            opposite: 0,
            conns: Vec::new(),
        });
        let res_b = self.ports.push(PortClass {
            name: names.1.to_string(),
            slot: slots.1,
            opposite: res_a,
            conns: Vec::new(),
        });
        self.ports[res_a].opposite = res_b;
        (res_a, res_b)
    }
    fn make_tile_slot(&mut self, name: &str) -> usize {
        self.tile_slots.push(name.to_string())
    }
    fn make_tile(&mut self, name: &str, cells: &[(usize, usize, usize)]) -> usize {
        self.tiles.push(TileClass {
            name: name.to_string(),
            cells: cells.iter().copied().collect(),
            muxes: Vec::new(),
            multimuxes: Vec::new(),
            trans: Vec::new(),
            sites: Vec::new(),
        })
    }
    fn make_tile_single(&mut self, name: &str, slot: usize) -> usize {
        self.make_tile(name, &[(0, 0, slot)])
    }

    fn make_wire(&mut self, name: &str, cls: &str, has_multicell_drive: bool) -> usize {
        self.wires.push(WireClass {
            name: name.to_string(),
            cls: cls.to_string(),
            has_multicell_drive,
            conn: WireConn::Internal,
        })
    }

    fn make_hbus_wire(
        &mut self,
        name: &str,
        cls: &str,
        bus: usize,
        has_multicell_drive: bool,
    ) -> usize {
        self.wires.push(WireClass {
            name: name.to_string(),
            cls: cls.to_string(),
            has_multicell_drive,
            conn: WireConn::HorizBus(bus),
        })
    }

    fn make_vbus_wire(
        &mut self,
        name: &str,
        cls: &str,
        bus: usize,
        has_multicell_drive: bool,
    ) -> usize {
        self.wires.push(WireClass {
            name: name.to_string(),
            cls: cls.to_string(),
            has_multicell_drive,
            conn: WireConn::VertBus(bus),
        })
    }

    fn make_tie_wire(&mut self, name: &str, cls: &str, state: TieState) -> usize {
        self.wires.push(WireClass {
            name: name.to_string(),
            cls: cls.to_string(),
            has_multicell_drive: false,
            conn: WireConn::Tie(state),
        })
    }

    fn make_simple_pconn(
        &mut self,
        wire_down: usize,
        wire_up: usize,
        pcls_down: usize,
        pcls_up: usize,
    ) -> usize {
        let psd = self.ports[pcls_down].slot;
        let psu = self.ports[pcls_up].slot;

        let cvd = &mut self.ports[pcls_down].conns;
        let cd = cvd.len();
        cvd.push(PortConn::Remote(wire_down));

        let cvu = &mut self.ports[pcls_up].conns;
        let cu = cvu.len();
        cvu.push(PortConn::Remote(wire_up));

        assert!(cu == cd);

        let wd = &mut self.wires[wire_down];
        match &mut wd.conn {
            WireConn::Internal => {
                wd.conn = WireConn::Port {
                    up: Some((psu, cu)),
                    down: Vec::new(),
                }
            }
            WireConn::Port { up, .. } => {
                assert!(up.is_none());
                *up = Some((psu, cu));
            }
            _ => panic!("simple_pconn on bus wire"),
        }

        let wu = &mut self.wires[wire_up];
        match &mut wu.conn {
            WireConn::Internal => {
                wu.conn = WireConn::Port {
                    up: None,
                    down: vec![(psd, cd)],
                }
            }
            WireConn::Port { down, .. } => {
                down.push((psd, cd));
            }
            _ => panic!("simple_pconn on bus wire"),
        }

        cu
    }
}

pub trait GridBuilder {
    fn fill_port_term(&mut self, geomdb: &GeomDb, xy: (usize, usize), pc: usize);
    fn fill_port_pair(
        &mut self,
        geomb: &GeomDb,
        xya: (usize, usize),
        xyb: (usize, usize),
        pc: (usize, usize),
    );
    fn fill_tile(&mut self, geomdb: &GeomDb, xy: (usize, usize), tc: usize);
}

impl GridBuilder for Grid {
    fn fill_port_term(&mut self, geomdb: &GeomDb, xy: (usize, usize), pc: usize) {
        let pcls = &geomdb.ports[pc];
        assert!(pcls.opposite == pc);
        let port = Port { cls: pc, other: xy };
        let cell = &mut self.grid[xy];
        assert!(cell.ports[pcls.slot].is_none());
        cell.ports[pcls.slot] = Some(port);
    }

    fn fill_port_pair(
        &mut self,
        geomdb: &GeomDb,
        xya: (usize, usize),
        xyb: (usize, usize),
        pc: (usize, usize),
    ) {
        let pcls_a = &geomdb.ports[pc.0];
        let pcls_b = &geomdb.ports[pc.1];
        assert!(pcls_a.opposite == pc.1);
        assert!(pcls_b.opposite == pc.0);
        let port_a = Port {
            cls: pc.0,
            other: xyb,
        };
        let port_b = Port {
            cls: pc.1,
            other: xya,
        };
        let cell_a = &mut self.grid[xya];
        assert!(cell_a.ports[pcls_a.slot].is_none());
        cell_a.ports[pcls_a.slot] = Some(port_a);
        let cell_b = &mut self.grid[xyb];
        assert!(cell_b.ports[pcls_b.slot].is_none());
        cell_b.ports[pcls_b.slot] = Some(port_b);
    }

    fn fill_tile(&mut self, geomdb: &GeomDb, xy: (usize, usize), tc: usize) {
        let tcls = &geomdb.tiles[tc];
        let tile = Tile {
            cls: tc,
            origin: xy,
        };
        let idx = self.tiles.len();
        self.tiles.push(tile);
        for (ci, (dx, dy, slot)) in tcls.cells.iter().copied().enumerate() {
            let cell = &mut self.grid[(xy.0 + dx, xy.1 + dy)];
            assert!(cell.tiles[slot].is_none());
            cell.tiles[slot] = Some((idx, ci));
        }
    }
}
