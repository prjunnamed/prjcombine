use crate::db::{IntDb, IntfInfo, PinDir, TermInfo, WireKind, IntfWireOutNaming, IntfWireInNaming, TermWireOutNaming, TermWireInFarNaming};
use prjcombine_entity::EntityId;
use std::collections::BTreeMap;

impl IntDb {
    pub fn print(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "INTDB {f}", f = self.name)?;
        for w in self.wires.values() {
            write!(o, "\tWIRE {n:14} ", n = w.name)?;
            match w.kind {
                WireKind::Tie0 => write!(o, "TIE_0")?,
                WireKind::Tie1 => write!(o, "TIE_1")?,
                WireKind::TiePullup => write!(o, "TIE_PULLUP")?,
                WireKind::ClkOut(i) => write!(o, "CLKOUT {i}")?,
                WireKind::MuxOut => write!(o, "MUXOUT")?,
                WireKind::LogicOut => write!(o, "LOGICOUT")?,
                WireKind::TestOut => write!(o, "TESTOUT")?,
                WireKind::MultiOut => write!(o, "MULTIOUT")?,
                WireKind::PipOut => write!(o, "PIPOUT")?,
                WireKind::Buf(bw) => write!(o, "BUF {bwn}", bwn = self.wires[bw].name)?,
                WireKind::Branch(d) => write!(o, "BRANCH {d}")?,
                WireKind::PipBranch(d) => write!(o, "PIPBRANCH {d}")?,
                WireKind::MultiBranch(d) => write!(o, "MULTIBRANCH {d}")?,
                WireKind::CondAlias(nk, ow) => write!(
                    o,
                    "CONDALIAS {nkn} {own}",
                    nkn = self.nodes.key(nk),
                    own = self.wires[ow].name
                )?,
            }
            writeln!(o)?;
        }
        for (_, name, node) in &self.nodes {
            writeln!(o, "\tNODE {name} {nt}", nt = node.tiles.len())?;
            for (&wo, mux) in &node.muxes {
                write!(
                    o,
                    "\t\tMUX {wot}.{won:14} <-",
                    wot = wo.0.to_idx(),
                    won = self.wires[wo.1].name
                )?;
                for &wi in &mux.ins {
                    write!(
                        o,
                        " {wit}.{win:14}",
                        wit = wi.0.to_idx(),
                        win = self.wires[wi.1].name
                    )?;
                }
                writeln!(o)?;
            }
            for (&wo, intf) in &node.intfs {
                match intf {
                    IntfInfo::InputDelay => {
                        writeln!(
                            o,
                            "\t\tINTF.DELAY {wot}.{won}",
                            wot = wo.0.to_idx(),
                            won = self.wires[wo.1].name
                        )?;
                    }
                    IntfInfo::OutputTestMux(ins) => {
                        write!(
                            o,
                            "\t\tINTF.TESTMUX {wot}.{won} <-",
                            wot = wo.0.to_idx(),
                            won = self.wires[wo.1].name
                        )?;
                        for &wi in ins {
                            write!(
                                o,
                                " {wit}.{win}",
                                wit = wi.0.to_idx(),
                                win = self.wires[wi.1].name
                            )?;
                        }
                        writeln!(o)?;
                    }
                }
            }
            let mut wires: BTreeMap<_, Vec<_>> = BTreeMap::new();
            for (bid, name, bel) in &node.bels {
                writeln!(o, "\t\tBEL {bid}: {name}", bid = bid.to_idx())?;
                for (pn, pin) in &bel.pins {
                    write!(
                        o,
                        "\t\t\t{d}{intf} {pn:20}",
                        d = match pin.dir {
                            PinDir::Input => " INPUT",
                            PinDir::Output => "OUTPUT",
                        },
                        intf = if pin.is_intf_in { ".INTF" } else { "     " }
                    )?;
                    for &wi in &pin.wires {
                        wires.entry(wi).or_default().push((name, pn));
                        write!(
                            o,
                            " {wit}.{win}",
                            wit = wi.0.to_idx(),
                            win = self.wires[wi.1].name
                        )?;
                    }
                    writeln!(o)?;
                }
            }
            for (wire, bels) in wires {
                write!(
                    o,
                    "\t\tWIRE {wt:3}.{wn:20}",
                    wt = wire.0.to_idx(),
                    wn = self.wires[wire.1].name
                )?;
                for (bel, pin) in bels {
                    write!(o, " {bel}.{pin}")?;
                }
                writeln!(o)?;
            }
        }
        for (_, name, term) in &self.terms {
            writeln!(o, "\tTERM {name} {d}", d = term.dir)?;
            for (w, ti) in &term.wires {
                let wn = &self.wires[w].name;
                match ti {
                    TermInfo::BlackHole => {
                        writeln!(o, "\t\tBLACKHOLE {wn}")?;
                    }
                    &TermInfo::PassNear(ow) => {
                        writeln!(o, "\t\tPASS NEAR {wn} <- {own}", own = self.wires[ow].name)?;
                    }
                    &TermInfo::PassFar(ow) => {
                        writeln!(o, "\t\tPASS FAR {wn} <- {own}", own = self.wires[ow].name)?;
                    }
                }
            }
        }
        for (_, name, naming) in &self.node_namings {
            writeln!(o, "\tNODE NAMING {name}")?;
            for (k, v) in &naming.wires {
                writeln!(
                    o,
                    "\t\tWIRE {wt:3}.{wn:20} {v}",
                    wt = k.0.to_idx(),
                    wn = self.wires[k.1].name
                )?;
            }
            for (k, v) in &naming.wire_bufs {
                writeln!(
                    o,
                    "\t\tWIRE BUF {wt:3}.{wn:20}: RT.{vrt} {vt} <- {vf}",
                    wt = k.0.to_idx(),
                    wn = self.wires[k.1].name,
                    vrt = v.tile.to_idx(),
                    vt = v.wire_to,
                    vf = v.wire_from,
                )?;
            }
            for (k, v) in &naming.ext_pips {
                writeln!(
                    o,
                    "\t\tEXT PIP {wtt:3}.{wtn:20} <- {wft:3}.{wfn:20}: RT.{vrt} {vt} <- {vf}",
                    wtt = k.0 .0.to_idx(),
                    wtn = self.wires[k.0 .1].name,
                    wft = k.1 .0.to_idx(),
                    wfn = self.wires[k.1 .1].name,
                    vrt = v.tile.to_idx(),
                    vt = v.wire_to,
                    vf = v.wire_from,
                )?;
            }
            for (bid, bn) in &naming.bels {
                writeln!(
                    o,
                    "\t\tBEL {bid} RT.{rt}:",
                    bid = bid.to_idx(),
                    rt = bn.tile.to_idx()
                )?;
                for (k, v) in &bn.pins {
                    write!(o, "\t\t\tPIN {k}: ")?;
                    if v.name == v.name_far {
                        write!(o, "{n}", n = v.name)?;
                    } else {
                        write!(o, "NEAR {nn} FAR {nf}", nn = v.name, nf = v.name_far)?;
                    }
                    if v.is_intf_out {
                        write!(o, " INTF.OUT")?;
                    }
                    writeln!(o)?;
                    for pip in &v.pips {
                        writeln!(
                            o,
                            "\t\t\t\tPIP RT.{rt} {wt} <- {wf}",
                            rt = pip.tile.to_idx(),
                            wt = pip.wire_to,
                            wf = pip.wire_from
                        )?;
                    }
                    for (w, pip) in &v.int_pips {
                        writeln!(
                            o,
                            "\t\t\t\tINT PIP {wt:3}.{wn:20}: RT.{rt} {pt} <- {pf}",
                            wt = w.0.to_idx(),
                            wn = self.wires[w.1].name,
                            rt = pip.tile.to_idx(),
                            pt = pip.wire_to,
                            pf = pip.wire_from
                        )?;
                    }
                }
            }
            for (w, wn) in &naming.intf_wires_out {
                write!(o, "\t\tINTF.OUT {wt:3}.{wn:20}: ", wt = w.0.to_idx(), wn = self.wires[w.1].name)?;
                match wn {
                    IntfWireOutNaming::Simple(n) => writeln!(o, "SIMPLE {n}")?,
                    IntfWireOutNaming::Buf(nt, nf) => writeln!(o, "BUF {nt} <- {nf}")?,
                }
            }
            for (w, wn) in &naming.intf_wires_in {
                write!(o, "\t\tINTF.IN {wt:3}.{wn:20}: ", wt = w.0.to_idx(), wn = self.wires[w.1].name)?;
                match wn {
                    IntfWireInNaming::Simple(n) => writeln!(o, "SIMPLE {n}")?,
                    IntfWireInNaming::Buf(nt, nf) => writeln!(o, "BUF {nt} <- {nf}")?,
                    IntfWireInNaming::TestBuf(nt, nf) => writeln!(o, "TESTBUF {nt} <- {nf}")?,
                    IntfWireInNaming::Delay(nt, nd, nf) => writeln!(o, "DELAY {nt} <- {nd} <- {nf}")?,
                }
            }
        }
        for (_, name, naming) in &self.term_namings {
            writeln!(o, "\tTERM NAMING {name}")?;
            for (w, wn) in &naming.wires_out {
                write!(o, "\t\tWIRE OUT {w}: ", w = self.wires[w].name)?;
                match wn {
                    TermWireOutNaming::Simple(n) => writeln!(o, "{n}")?,
                    TermWireOutNaming::Buf(nt, nf) => writeln!(o, "{nt} <- {nf}")?,
                }
            }
            for (w, wn) in &naming.wires_in_near {
                writeln!(o, "\t\tWIRE IN NEAR {w}: {wn}", w = self.wires[w].name)?;
            }
            for (w, wn) in &naming.wires_in_far {
                write!(o, "\t\tWIRE IN FAR {w}: ", w = self.wires[w].name)?;
                match wn {
                    TermWireInFarNaming::Simple(n) => writeln!(o, "{n}")?,
                    TermWireInFarNaming::Buf(nt, nf) => writeln!(o, "{nt} <- {nf}")?,
                    TermWireInFarNaming::BufFar(nt, nm, nf) => writeln!(o, "{nt} <- {nm} <- {nf}")?,
                }
            }
        }
        Ok(())
    }
}
