use std::collections::{BTreeMap, btree_map};

use prjcombine_re_sdf::{Delay, Edge, Sdf};

pub fn set_timing(tgt: &mut BTreeMap<String, i64>, name: &str, src: i64) {
    match tgt.entry(name.into()) {
        btree_map::Entry::Occupied(e) => assert_eq!(*e.get(), src),
        btree_map::Entry::Vacant(e) => {
            e.insert(src);
        }
    }
}

fn extract_delay(del: Delay, tgt: &mut BTreeMap<String, i64>, tname: &str) {
    assert_eq!(del.min, del.typ);
    assert_eq!(del.min, del.max);
    assert!(del.min.is_integer());
    let n: i64 = del.min.try_into().unwrap();
    set_timing(tgt, tname, n);
}

pub fn extract_buf(sdf: &Sdf, name: &str, tgt: &mut BTreeMap<String, i64>, tname: &str) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_BUF");
    assert_eq!(cell.iopath.len(), 1);
    let iop = &cell.iopath[0];
    assert_eq!(iop.port_from, Edge::Plain("I".into()));
    assert_eq!(iop.port_to, Edge::Plain("O".into()));
    assert_eq!(iop.del_rise, iop.del_fall);
    extract_delay(iop.del_rise, tgt, tname);
}

pub fn extract_tri_i(sdf: &Sdf, name: &str, tgt: &mut BTreeMap<String, i64>, tname: &str) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_TRI");
    assert_eq!(cell.ports.len(), 2);
    let p0 = &cell.ports[0];
    assert_eq!(p0.port, "I");
    assert_eq!(p0.del_rise, p0.del_fall);
    extract_delay(p0.del_rise, tgt, tname);
}

pub fn extract_tri_ctl(sdf: &Sdf, name: &str, tgt: &mut BTreeMap<String, i64>, tname: &str) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_TRI");
    assert_eq!(cell.ports.len(), 2);
    let p1 = &cell.ports[1];
    assert_eq!(p1.port, "CTL");
    assert_eq!(p1.del_rise, p1.del_fall);
    extract_delay(p1.del_rise, tgt, tname);
}

pub fn extract_and2(sdf: &Sdf, name: &str, tgt: &mut BTreeMap<String, i64>, tname: &str) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_AND2");
    assert_eq!(cell.ports.len(), 2);
    let p0 = &cell.ports[0];
    let p1 = &cell.ports[1];
    assert_eq!(p0.port, "I0");
    assert_eq!(p1.port, "I1");
    assert_eq!(p0.del_rise, p0.del_fall);
    assert_eq!(p1.del_rise, p1.del_fall);
    assert_eq!(p0.del_rise, p1.del_rise);
    extract_delay(p0.del_rise, tgt, tname);
}

pub fn extract_and2_iopath(sdf: &Sdf, name: &str, tgt: &mut BTreeMap<String, i64>, tname: &str) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_AND2");
    assert_eq!(cell.iopath.len(), 2);
    let iop0 = &cell.iopath[0];
    let iop1 = &cell.iopath[1];
    assert_eq!(iop0.port_from, Edge::Plain("I0".into()));
    assert_eq!(iop0.port_to, Edge::Plain("O".into()));
    assert_eq!(iop1.port_from, Edge::Plain("I1".into()));
    assert_eq!(iop1.port_to, Edge::Plain("O".into()));
    assert_eq!(iop0.del_rise, iop0.del_fall);
    assert_eq!(iop1.del_rise, iop1.del_fall);
    assert_eq!(iop0.del_rise, iop1.del_rise);
    extract_delay(iop0.del_rise, tgt, tname);
}

#[allow(clippy::too_many_arguments)]
pub fn extract_ff(
    sdf: &Sdf,
    name: &str,
    tgt: &mut BTreeMap<String, i64>,
    tname_del_clk_q: &str,
    tname_del_sr_q: &str,
    tname_setup_d_clk: &str,
    tname_hold_d_clk: &str,
    tname_setup_ce_clk: Option<&str>,
    tname_hold_ce_clk: Option<&str>,
    tname_width_clk: &str,
    tname_width_sr: &str,
) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_FF");
    assert_eq!(cell.iopath.len(), 3);
    let iop0 = &cell.iopath[0];
    assert_eq!(iop0.port_from, Edge::Plain("CLK".into()));
    assert_eq!(iop0.port_to, Edge::Plain("O".into()));
    assert_eq!(iop0.del_rise, iop0.del_fall);
    extract_delay(iop0.del_rise, tgt, tname_del_clk_q);
    let iop1 = &cell.iopath[1];
    assert_eq!(iop1.port_from, Edge::Plain("SET".into()));
    assert_eq!(iop1.port_to, Edge::Plain("O".into()));
    assert_eq!(iop1.del_rise, iop1.del_fall);
    extract_delay(iop1.del_rise, tgt, tname_del_sr_q);
    let iop2 = &cell.iopath[2];
    assert_eq!(iop2.port_from, Edge::Plain("RST".into()));
    assert_eq!(iop2.port_to, Edge::Plain("O".into()));
    assert_eq!(iop2.del_rise, iop2.del_fall);
    extract_delay(iop2.del_rise, tgt, tname_del_sr_q);

    assert_eq!(cell.setuphold.len(), 3);
    let sh0 = &cell.setuphold[0];
    assert_eq!(sh0.edge_d, Edge::Posedge("I".into()));
    assert_eq!(sh0.edge_c, Edge::Posedge("CLK".into()));
    extract_delay(sh0.setup.unwrap(), tgt, tname_setup_d_clk);
    extract_delay(sh0.hold.unwrap(), tgt, tname_hold_d_clk);
    let sh1 = &cell.setuphold[1];
    assert_eq!(sh1.edge_d, Edge::Negedge("I".into()));
    assert_eq!(sh1.edge_c, Edge::Posedge("CLK".into()));
    extract_delay(sh1.setup.unwrap(), tgt, tname_setup_d_clk);
    extract_delay(sh1.hold.unwrap(), tgt, tname_hold_d_clk);
    let sh2 = &cell.setuphold[2];
    assert_eq!(sh2.edge_d, Edge::Posedge("CE".into()));
    assert_eq!(sh2.edge_c, Edge::Posedge("CLK".into()));
    if let Some(tname) = tname_setup_ce_clk {
        extract_delay(sh2.setup.unwrap(), tgt, tname);
    }
    if let Some(tname) = tname_hold_ce_clk {
        extract_delay(sh2.hold.unwrap(), tgt, tname);
    }

    assert_eq!(cell.period.len(), 1);
    let per0 = &cell.period[0];
    assert_eq!(per0.edge, Edge::Posedge("CLK".into()));
    let mut tmp = BTreeMap::new();
    extract_delay(per0.val, &mut tmp, "TMP");
    set_timing(tgt, tname_width_clk, tmp["TMP"] / 2);

    assert_eq!(cell.width.len(), 2);
    let w0 = &cell.width[0];
    assert_eq!(w0.edge, Edge::Posedge("SET".into()));
    extract_delay(w0.val, tgt, tname_width_sr);
    let w1 = &cell.width[1];
    assert_eq!(w1.edge, Edge::Posedge("RST".into()));
    extract_delay(w1.val, tgt, tname_width_sr);
}

#[allow(clippy::too_many_arguments)]
pub fn extract_latch(
    sdf: &Sdf,
    name: &str,
    tgt: &mut BTreeMap<String, i64>,
    tname_del_d_q: &str,
    tname_del_clk_q: &str,
    tname_del_sr_q: Option<&str>,
    tname_setup_d_clk: &str,
    tname_hold_d_clk: &str,
    tname_width_clk: &str,
    tname_width_sr: Option<&str>,
) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_LATCHE");
    assert_eq!(cell.iopath.len(), 5);
    let iop0 = &cell.iopath[0];
    assert_eq!(iop0.port_from, Edge::Plain("I".into()));
    assert_eq!(iop0.port_to, Edge::Plain("O".into()));
    assert_eq!(iop0.del_rise, iop0.del_fall);
    extract_delay(iop0.del_rise, tgt, tname_del_d_q);
    let iop2 = &cell.iopath[2];
    assert_eq!(iop2.port_from, Edge::Plain("CLK".into()));
    assert_eq!(iop2.port_to, Edge::Plain("O".into()));
    assert_eq!(iop2.del_rise, iop2.del_fall);
    extract_delay(iop2.del_rise, tgt, tname_del_clk_q);
    let iop3 = &cell.iopath[3];
    assert_eq!(iop3.port_from, Edge::Plain("SET".into()));
    assert_eq!(iop3.port_to, Edge::Plain("O".into()));
    assert_eq!(iop3.del_rise, iop3.del_fall);
    let iop4 = &cell.iopath[4];
    assert_eq!(iop4.port_from, Edge::Plain("RST".into()));
    assert_eq!(iop4.port_to, Edge::Plain("O".into()));
    assert_eq!(iop4.del_rise, iop4.del_fall);
    if let Some(tname) = tname_del_sr_q {
        extract_delay(iop3.del_rise, tgt, tname);
        extract_delay(iop4.del_rise, tgt, tname);
    }

    assert_eq!(cell.setuphold.len(), 3);
    let sh0 = &cell.setuphold[0];
    assert_eq!(sh0.edge_d, Edge::Posedge("I".into()));
    assert_eq!(sh0.edge_c, Edge::Negedge("CLK".into()));
    extract_delay(sh0.setup.unwrap(), tgt, tname_setup_d_clk);
    extract_delay(sh0.hold.unwrap(), tgt, tname_hold_d_clk);
    let sh1 = &cell.setuphold[1];
    assert_eq!(sh1.edge_d, Edge::Negedge("I".into()));
    assert_eq!(sh1.edge_c, Edge::Negedge("CLK".into()));
    extract_delay(sh1.setup.unwrap(), tgt, tname_setup_d_clk);
    extract_delay(sh1.hold.unwrap(), tgt, tname_hold_d_clk);

    assert_eq!(cell.width.len(), 3);
    let w0 = &cell.width[0];
    assert_eq!(w0.edge, Edge::Posedge("CLK".into()));
    extract_delay(w0.val, tgt, tname_width_clk);
    let w1 = &cell.width[1];
    assert_eq!(w1.edge, Edge::Posedge("SET".into()));
    let w2 = &cell.width[2];
    assert_eq!(w2.edge, Edge::Posedge("RST".into()));
    if let Some(tname) = tname_width_sr {
        extract_delay(w1.val, tgt, tname);
        extract_delay(w2.val, tgt, tname);
    }
}
