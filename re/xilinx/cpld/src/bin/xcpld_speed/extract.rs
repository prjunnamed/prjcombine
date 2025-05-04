use std::collections::btree_map;

use prjcombine_re_sdf::{self as sdf, Edge, Sdf};
use prjcombine_types::speed::{Scalar, SetupHold, Speed, SpeedVal, Time};

pub fn set_timing(tgt: &mut Speed, name: &str, src: SpeedVal) {
    match tgt.vals.entry(name.into()) {
        btree_map::Entry::Occupied(e) => assert_eq!(*e.get(), src),
        btree_map::Entry::Vacant(e) => {
            e.insert(src);
        }
    }
}

pub fn set_timing_delay(tgt: &mut Speed, name: &str, src: Time) {
    set_timing(tgt, name, SpeedVal::Delay(src));
}

fn convert_delay(del: sdf::Delay) -> Time {
    assert_eq!(del.min, del.typ);
    assert_eq!(del.min, del.max);
    del.min
}

fn collect_delay(del: sdf::Delay, tgt: &mut Speed, tname: &str) {
    let del = convert_delay(del);
    set_timing(tgt, tname, SpeedVal::Delay(del));
}

fn collect_setuphold(sh: &sdf::SetupHold, tgt: &mut Speed, tname: &str) {
    let setup = convert_delay(sh.setup.unwrap());
    let hold = convert_delay(sh.hold.unwrap());
    set_timing(tgt, tname, SpeedVal::SetupHold(SetupHold { setup, hold }));
}

pub fn extract_buf(sdf: &Sdf, name: &str) -> Time {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_BUF");
    assert_eq!(cell.iopath.len(), 1);
    let iop = &cell.iopath[0];
    assert_eq!(iop.port_from, Edge::Plain("I".into()));
    assert_eq!(iop.port_to, Edge::Plain("O".into()));
    assert_eq!(iop.del_rise, iop.del_fall);
    convert_delay(iop.del_rise)
}

pub fn collect_buf(sdf: &Sdf, name: &str, tgt: &mut Speed, tname: &str) {
    set_timing_delay(tgt, tname, extract_buf(sdf, name));
}

pub fn collect_tri_i(sdf: &Sdf, name: &str, tgt: &mut Speed, tname: &str) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_TRI");
    assert_eq!(cell.ports.len(), 2);
    let p0 = &cell.ports[0];
    assert_eq!(p0.port, "I");
    assert_eq!(p0.del_rise, p0.del_fall);
    collect_delay(p0.del_rise, tgt, tname);
}

pub fn collect_tri_ctl(sdf: &Sdf, name: &str, tgt: &mut Speed, tname: &str) {
    let cell = &sdf.cells_by_name[name];
    assert_eq!(cell.typ, "X_TRI");
    assert_eq!(cell.ports.len(), 2);
    let p1 = &cell.ports[1];
    assert_eq!(p1.port, "CTL");
    assert_eq!(p1.del_rise, p1.del_fall);
    collect_delay(p1.del_rise, tgt, tname);
}

pub fn extract_and2(sdf: &Sdf, name: &str) -> Time {
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
    convert_delay(p0.del_rise)
}

pub fn collect_and2(sdf: &Sdf, name: &str, tgt: &mut Speed, tname: &str) {
    set_timing_delay(tgt, tname, extract_and2(sdf, name));
}

pub fn collect_and2_iopath(sdf: &Sdf, name: &str, tgt: &mut Speed, tname: &str) {
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
    collect_delay(iop0.del_rise, tgt, tname);
}

#[allow(clippy::too_many_arguments)]
pub fn collect_ff(
    sdf: &Sdf,
    name: &str,
    tgt: &mut Speed,
    tname_del_clk_q: &str,
    tname_del_sr_q: &str,
    tname_sh_d_clk: &str,
    tname_sh_ce_clk: Option<&str>,
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
    collect_delay(iop0.del_rise, tgt, tname_del_clk_q);
    let iop1 = &cell.iopath[1];
    assert_eq!(iop1.port_from, Edge::Plain("SET".into()));
    assert_eq!(iop1.port_to, Edge::Plain("O".into()));
    assert_eq!(iop1.del_rise, iop1.del_fall);
    collect_delay(iop1.del_rise, tgt, tname_del_sr_q);
    let iop2 = &cell.iopath[2];
    assert_eq!(iop2.port_from, Edge::Plain("RST".into()));
    assert_eq!(iop2.port_to, Edge::Plain("O".into()));
    assert_eq!(iop2.del_rise, iop2.del_fall);
    collect_delay(iop2.del_rise, tgt, tname_del_sr_q);

    assert_eq!(cell.setuphold.len(), 3);
    let sh0 = &cell.setuphold[0];
    assert_eq!(sh0.edge_d, Edge::Posedge("I".into()));
    assert_eq!(sh0.edge_c, Edge::Posedge("CLK".into()));
    collect_setuphold(sh0, tgt, tname_sh_d_clk);
    let sh1 = &cell.setuphold[1];
    assert_eq!(sh1.edge_d, Edge::Negedge("I".into()));
    assert_eq!(sh1.edge_c, Edge::Posedge("CLK".into()));
    collect_setuphold(sh1, tgt, tname_sh_d_clk);
    let sh2 = &cell.setuphold[2];
    assert_eq!(sh2.edge_d, Edge::Posedge("CE".into()));
    assert_eq!(sh2.edge_c, Edge::Posedge("CLK".into()));
    if let Some(tname) = tname_sh_ce_clk {
        collect_setuphold(sh2, tgt, tname);
    }

    assert_eq!(cell.period.len(), 1);
    let per0 = &cell.period[0];
    assert_eq!(per0.edge, Edge::Posedge("CLK".into()));
    let period = convert_delay(per0.val);
    set_timing(
        tgt,
        tname_width_clk,
        SpeedVal::PulseWidth(Time(period.0 / Scalar(2.0))),
    );

    assert_eq!(cell.width.len(), 2);
    let w0 = &cell.width[0];
    assert_eq!(w0.edge, Edge::Posedge("SET".into()));
    let width = convert_delay(w0.val);
    set_timing(tgt, tname_width_sr, SpeedVal::PulseWidth(width));
    let w1 = &cell.width[1];
    assert_eq!(w1.edge, Edge::Posedge("RST".into()));
    let width = convert_delay(w1.val);
    set_timing(tgt, tname_width_sr, SpeedVal::PulseWidth(width));
}

#[allow(clippy::too_many_arguments)]
pub fn collect_latch(
    sdf: &Sdf,
    name: &str,
    tgt: &mut Speed,
    tname_del_d_q: &str,
    tname_del_clk_q: &str,
    tname_del_sr_q: Option<&str>,
    tname_sh_d_clk: &str,
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
    collect_delay(iop0.del_rise, tgt, tname_del_d_q);
    let iop2 = &cell.iopath[2];
    assert_eq!(iop2.port_from, Edge::Plain("CLK".into()));
    assert_eq!(iop2.port_to, Edge::Plain("O".into()));
    assert_eq!(iop2.del_rise, iop2.del_fall);
    collect_delay(iop2.del_rise, tgt, tname_del_clk_q);
    let iop3 = &cell.iopath[3];
    assert_eq!(iop3.port_from, Edge::Plain("SET".into()));
    assert_eq!(iop3.port_to, Edge::Plain("O".into()));
    assert_eq!(iop3.del_rise, iop3.del_fall);
    let iop4 = &cell.iopath[4];
    assert_eq!(iop4.port_from, Edge::Plain("RST".into()));
    assert_eq!(iop4.port_to, Edge::Plain("O".into()));
    assert_eq!(iop4.del_rise, iop4.del_fall);
    if let Some(tname) = tname_del_sr_q {
        collect_delay(iop3.del_rise, tgt, tname);
        collect_delay(iop4.del_rise, tgt, tname);
    }

    assert_eq!(cell.setuphold.len(), 3);
    let sh0 = &cell.setuphold[0];
    assert_eq!(sh0.edge_d, Edge::Posedge("I".into()));
    assert_eq!(sh0.edge_c, Edge::Negedge("CLK".into()));
    collect_setuphold(sh0, tgt, tname_sh_d_clk);
    let sh1 = &cell.setuphold[1];
    assert_eq!(sh1.edge_d, Edge::Negedge("I".into()));
    assert_eq!(sh1.edge_c, Edge::Negedge("CLK".into()));
    collect_setuphold(sh1, tgt, tname_sh_d_clk);

    assert_eq!(cell.width.len(), 3);
    let w0 = &cell.width[0];
    assert_eq!(w0.edge, Edge::Posedge("CLK".into()));
    set_timing(
        tgt,
        tname_width_clk,
        SpeedVal::PulseWidth(convert_delay(w0.val)),
    );
    let w1 = &cell.width[1];
    assert_eq!(w1.edge, Edge::Posedge("SET".into()));
    let w2 = &cell.width[2];
    assert_eq!(w2.edge, Edge::Posedge("RST".into()));
    if let Some(tname) = tname_width_sr {
        set_timing(tgt, tname, SpeedVal::PulseWidth(convert_delay(w1.val)));
        set_timing(tgt, tname, SpeedVal::PulseWidth(convert_delay(w2.val)));
    }
}
