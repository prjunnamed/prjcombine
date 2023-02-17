use prjcombine_sdf::{Delay, Edge, Sdf};

pub fn set_timing(tgt: &mut Option<i64>, src: i64) {
    if let Some(cur) = *tgt {
        assert_eq!(cur, src);
    } else {
        *tgt = Some(src);
    }
}

fn extract_delay(del: Delay, tgt: &mut Option<i64>) {
    assert_eq!(del.min, del.typ);
    assert_eq!(del.min, del.max);
    set_timing(tgt, del.min);
}

pub fn extract_buf(sdf: &Sdf, name: &str, tgt: &mut Option<i64>) {
    let cell = &sdf.cells[name];
    assert_eq!(cell.typ, "X_BUF");
    assert_eq!(cell.iopath.len(), 1);
    let iop = &cell.iopath[0];
    assert_eq!(iop.port_from, "I");
    assert_eq!(iop.port_to, "O");
    assert_eq!(iop.del_rise, iop.del_fall);
    extract_delay(iop.del_rise, tgt);
}

pub fn extract_tri_i(sdf: &Sdf, name: &str, tgt: &mut Option<i64>) {
    let cell = &sdf.cells[name];
    assert_eq!(cell.typ, "X_TRI");
    assert_eq!(cell.ports.len(), 2);
    let p0 = &cell.ports[0];
    assert_eq!(p0.port, "I");
    assert_eq!(p0.del_rise, p0.del_fall);
    extract_delay(p0.del_rise, tgt);
}

pub fn extract_tri_ctl(sdf: &Sdf, name: &str, tgt: &mut Option<i64>) {
    let cell = &sdf.cells[name];
    assert_eq!(cell.typ, "X_TRI");
    assert_eq!(cell.ports.len(), 2);
    let p1 = &cell.ports[1];
    assert_eq!(p1.port, "CTL");
    assert_eq!(p1.del_rise, p1.del_fall);
    extract_delay(p1.del_rise, tgt);
}

pub fn extract_and2(sdf: &Sdf, name: &str, tgt: &mut Option<i64>) {
    let cell = &sdf.cells[name];
    assert_eq!(cell.typ, "X_AND2");
    assert_eq!(cell.ports.len(), 2);
    let p0 = &cell.ports[0];
    let p1 = &cell.ports[1];
    assert_eq!(p0.port, "I0");
    assert_eq!(p1.port, "I1");
    assert_eq!(p0.del_rise, p0.del_fall);
    assert_eq!(p1.del_rise, p1.del_fall);
    assert_eq!(p0.del_rise, p1.del_rise);
    extract_delay(p0.del_rise, tgt);
}

pub fn extract_and2_iopath(sdf: &Sdf, name: &str, tgt: &mut Option<i64>) {
    let cell = &sdf.cells[name];
    assert_eq!(cell.typ, "X_AND2");
    assert_eq!(cell.iopath.len(), 2);
    let iop0 = &cell.iopath[0];
    let iop1 = &cell.iopath[1];
    assert_eq!(iop0.port_from, "I0");
    assert_eq!(iop0.port_to, "O");
    assert_eq!(iop1.port_from, "I1");
    assert_eq!(iop1.port_to, "O");
    assert_eq!(iop0.del_rise, iop0.del_fall);
    assert_eq!(iop1.del_rise, iop1.del_fall);
    assert_eq!(iop0.del_rise, iop1.del_rise);
    extract_delay(iop0.del_rise, tgt);
}

#[allow(clippy::too_many_arguments)]
pub fn extract_ff(
    sdf: &Sdf,
    name: &str,
    tgt_del_clk_q: &mut Option<i64>,
    tgt_del_sr_q: &mut Option<i64>,
    tgt_setup_d_clk: &mut Option<i64>,
    tgt_hold_d_clk: &mut Option<i64>,
    tgt_setup_ce_clk: &mut Option<i64>,
    tgt_hold_ce_clk: &mut Option<i64>,
    tgt_period_clk: &mut Option<i64>,
    tgt_width_sr: &mut Option<i64>,
) {
    let cell = &sdf.cells[name];
    assert_eq!(cell.typ, "X_FF");
    assert_eq!(cell.iopath.len(), 3);
    let iop0 = &cell.iopath[0];
    assert_eq!(iop0.port_from, "CLK");
    assert_eq!(iop0.port_to, "O");
    assert_eq!(iop0.del_rise, iop0.del_fall);
    extract_delay(iop0.del_rise, tgt_del_clk_q);
    let iop1 = &cell.iopath[1];
    assert_eq!(iop1.port_from, "SET");
    assert_eq!(iop1.port_to, "O");
    assert_eq!(iop1.del_rise, iop1.del_fall);
    extract_delay(iop1.del_rise, tgt_del_sr_q);
    let iop2 = &cell.iopath[2];
    assert_eq!(iop2.port_from, "RST");
    assert_eq!(iop2.port_to, "O");
    assert_eq!(iop2.del_rise, iop2.del_fall);
    extract_delay(iop2.del_rise, tgt_del_sr_q);

    assert_eq!(cell.setuphold.len(), 3);
    let sh0 = &cell.setuphold[0];
    assert_eq!(sh0.edge_d, Edge::Posedge("I".into()));
    assert_eq!(sh0.edge_c, Edge::Posedge("CLK".into()));
    extract_delay(sh0.setup, tgt_setup_d_clk);
    extract_delay(sh0.hold, tgt_hold_d_clk);
    let sh1 = &cell.setuphold[1];
    assert_eq!(sh1.edge_d, Edge::Negedge("I".into()));
    assert_eq!(sh1.edge_c, Edge::Posedge("CLK".into()));
    extract_delay(sh1.setup, tgt_setup_d_clk);
    extract_delay(sh1.hold, tgt_hold_d_clk);
    let sh2 = &cell.setuphold[2];
    assert_eq!(sh2.edge_d, Edge::Posedge("CE".into()));
    assert_eq!(sh2.edge_c, Edge::Posedge("CLK".into()));
    extract_delay(sh2.setup, tgt_setup_ce_clk);
    extract_delay(sh2.hold, tgt_hold_ce_clk);

    assert_eq!(cell.period.len(), 1);
    let per0 = &cell.period[0];
    assert_eq!(per0.edge, Edge::Posedge("CLK".into()));
    extract_delay(per0.val, tgt_period_clk);

    assert_eq!(cell.width.len(), 2);
    let w0 = &cell.width[0];
    assert_eq!(w0.edge, Edge::Posedge("SET".into()));
    extract_delay(w0.val, tgt_width_sr);
    let w1 = &cell.width[1];
    assert_eq!(w1.edge, Edge::Posedge("RST".into()));
    extract_delay(w1.val, tgt_width_sr);
}

#[allow(clippy::too_many_arguments)]
pub fn extract_latch(
    sdf: &Sdf,
    name: &str,
    tgt_del_d_q: &mut Option<i64>,
    tgt_del_clk_q: &mut Option<i64>,
    tgt_del_sr_q: &mut Option<i64>,
    tgt_setup_d_clk: &mut Option<i64>,
    tgt_hold_d_clk: &mut Option<i64>,
    tgt_width_clk: &mut Option<i64>,
    tgt_width_sr: &mut Option<i64>,
) {
    let cell = &sdf.cells[name];
    assert_eq!(cell.typ, "X_LATCHE");
    assert_eq!(cell.iopath.len(), 5);
    let iop0 = &cell.iopath[0];
    assert_eq!(iop0.port_from, "I");
    assert_eq!(iop0.port_to, "O");
    assert_eq!(iop0.del_rise, iop0.del_fall);
    extract_delay(iop0.del_rise, tgt_del_d_q);
    let iop2 = &cell.iopath[2];
    assert_eq!(iop2.port_from, "CLK");
    assert_eq!(iop2.port_to, "O");
    assert_eq!(iop2.del_rise, iop2.del_fall);
    extract_delay(iop2.del_rise, tgt_del_clk_q);
    let iop3 = &cell.iopath[3];
    assert_eq!(iop3.port_from, "SET");
    assert_eq!(iop3.port_to, "O");
    assert_eq!(iop3.del_rise, iop3.del_fall);
    extract_delay(iop3.del_rise, tgt_del_sr_q);
    let iop4 = &cell.iopath[4];
    assert_eq!(iop4.port_from, "RST");
    assert_eq!(iop4.port_to, "O");
    assert_eq!(iop4.del_rise, iop4.del_fall);
    extract_delay(iop4.del_rise, tgt_del_sr_q);

    assert_eq!(cell.setuphold.len(), 3);
    let sh0 = &cell.setuphold[0];
    assert_eq!(sh0.edge_d, Edge::Posedge("I".into()));
    assert_eq!(sh0.edge_c, Edge::Negedge("CLK".into()));
    extract_delay(sh0.setup, tgt_setup_d_clk);
    extract_delay(sh0.hold, tgt_hold_d_clk);
    let sh1 = &cell.setuphold[1];
    assert_eq!(sh1.edge_d, Edge::Negedge("I".into()));
    assert_eq!(sh1.edge_c, Edge::Negedge("CLK".into()));
    extract_delay(sh1.setup, tgt_setup_d_clk);
    extract_delay(sh1.hold, tgt_hold_d_clk);

    assert_eq!(cell.width.len(), 3);
    let w0 = &cell.width[0];
    assert_eq!(w0.edge, Edge::Posedge("CLK".into()));
    extract_delay(w0.val, tgt_width_clk);
    let w1 = &cell.width[1];
    assert_eq!(w1.edge, Edge::Posedge("SET".into()));
    extract_delay(w1.val, tgt_width_sr);
    let w2 = &cell.width[2];
    assert_eq!(w2.edge, Edge::Posedge("RST".into()));
    extract_delay(w2.val, tgt_width_sr);
}
