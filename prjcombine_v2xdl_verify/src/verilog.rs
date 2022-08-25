use crate::types::{BitVal, ParamVal, SrcInst, Test};
use std::fmt::Write;

fn emit_pval(out: &mut String, val: &ParamVal) {
    match val {
        ParamVal::String(s) => {
            write!(out, "\"{s}\"").unwrap();
        }
        ParamVal::Int(iv) => {
            write!(out, "{iv}").unwrap();
        }
        ParamVal::Float(iv) => {
            write!(out, "{iv}").unwrap();
        }
        ParamVal::Bits(bv) => {
            let l = bv.len();
            write!(out, "{l}'b").unwrap();
            for b in bv.iter().rev() {
                match b {
                    BitVal::S0 => write!(out, "0").unwrap(),
                    BitVal::S1 => write!(out, "1").unwrap(),
                }
            }
        }
    }
}

fn emit_inst(out: &mut String, inst: &SrcInst) {
    for (n, v) in inst.attrs.iter() {
        write!(out, "(* {n} = ").unwrap();
        emit_pval(out, v);
        writeln!(out, " *)").unwrap();
    }
    write!(out, "{t}", t = inst.typ).unwrap();
    if !inst.params.is_empty() {
        writeln!(out, " #(").unwrap();
        for (i, (n, v)) in inst.params.iter().enumerate() {
            if i != 0 {
                writeln!(out, ",").unwrap();
            }
            write!(out, "    .{n}(").unwrap();
            emit_pval(out, v);
            write!(out, ")").unwrap();
        }
        writeln!(out, "").unwrap();
        write!(out, ")").unwrap();
    }
    writeln!(out, " {n} (", n = inst.name).unwrap();
    for (i, (n, v)) in inst.pins.iter().enumerate() {
        if i != 0 {
            writeln!(out, ",").unwrap();
        }
        write!(out, "    .{n}(").unwrap();
        if v.len() == 1 {
            write!(out, "{x}", x = v[0]).unwrap();
        } else {
            write!(out, "{{").unwrap();
            for (ii, w) in v.iter().rev().enumerate() {
                if ii != 0 {
                    write!(out, ", ").unwrap();
                }
                write!(out, "{w}").unwrap();
            }
            write!(out, "}}").unwrap();
        }
        write!(out, ")").unwrap();
    }
    writeln!(out, ");").unwrap();
    writeln!(out, "").unwrap();
}

pub fn emit(test: &Test) -> String {
    let mut res = String::new();
    writeln!(res, "`default_nettype none").unwrap();
    writeln!(res, "module top(I);").unwrap();
    writeln!(res, "input wire I;").unwrap();
    writeln!(res, "").unwrap();
    for w in test.src_wires.iter() {
        writeln!(res, "wire {w};").unwrap();
    }
    for w in test.src_ins.iter() {
        writeln!(res, "(* KEEP = \"TRUE\" *)").unwrap();
        writeln!(res, "(* S = \"TRUE\" *)").unwrap();
        writeln!(res, "wire _in_{w};").unwrap();
        writeln!(res, "(* KEEP = \"TRUE\" *)").unwrap();
        writeln!(res, "wire {w};").unwrap();
        writeln!(res, "BUF _ibuf2_{w} (.I(0), .O(_in_{w}));").unwrap();
        writeln!(res, "BUF _ibuf_{w} (.I(_in_{w}), .O({w}));").unwrap();
    }
    for w in test.src_outs.iter() {
        writeln!(res, "(* KEEP = \"TRUE\" *)").unwrap();
        writeln!(res, "wire {w};").unwrap();
        writeln!(res, "(* KEEP = \"TRUE\" *)").unwrap();
        writeln!(res, "(* S = \"TRUE\" *)").unwrap();
        writeln!(res, "wire _out_{w};").unwrap();
        writeln!(res, "BUF _obuf_{w} (.I({w}), .O(_out_{w}));").unwrap();
    }
    writeln!(res, "").unwrap();
    for it in test.src_insts.iter() {
        emit_inst(&mut res, it);
    }
    writeln!(res, "endmodule").unwrap();
    res
}
