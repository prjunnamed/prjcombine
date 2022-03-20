use structopt::StructOpt;
use prjcombine_toolchain::Toolchain;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use rayon::prelude::*;

mod types;
mod run;
mod verilog;
mod verify;
mod clb_lut4;
mod clb_lut6;
mod ramb;
mod dsp;
mod hard;
mod clkbuf;
mod cfg;

use types::{TestGenCtx, Test};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "prjcombine_v2xdl_verify",
    about = "Verify ISE Verilog -> XDL mapping."
)]
struct Opt {
    toolchain: String,
    families: Vec<String>,
}

fn get_virtex_tests(family: &str) -> Vec<Test> {
    let mut res = Vec::new();
    let mut ctx = TestGenCtx::new();
    let part = match family {
        "spartan2" => "xc2s200-6-fg456",
        "spartan2e" => "xc2s600e-6-fg676",
        "virtex" => "xcv1000-6-fg680",
        "virtexe" => "xcv1000e-6-fg680",
        _ => unreachable!(),
    };
    for i in 0..10 {
        let mut test = Test::new(&format!("clb{i}"), part);
        clb_lut4::gen_clb(&mut ctx, clb_lut4::Mode::Virtex, &mut test);
        ramb::gen_ramb(&mut ctx, ramb::Mode::Virtex, &mut test);
        clkbuf::gen_clkbuf(&mut test, &mut ctx, clkbuf::Mode::Virtex);
        cfg::gen_cfg(&mut test, &mut ctx, cfg::Mode::Virtex);
        res.push(test);
    }
    res
}

fn get_virtex2_tests(family: &str) -> Vec<Test> {
    let mut res = Vec::new();
    let mut ctx = TestGenCtx::new();
    let (part, cfg_mode) = match family {
        "virtex2" => ("xc2v4000-4-ff1517", cfg::Mode::Virtex2),
        "virtex2p" => ("xc2vp40-5-ff1152", cfg::Mode::Virtex2P),
        _ => unreachable!(),
    };
    for i in 0..10 {
        let mut test = Test::new(&format!("clb{i}"), part);
        clb_lut4::gen_clb(&mut ctx, clb_lut4::Mode::Virtex2, &mut test);
        ramb::gen_ramb(&mut ctx, ramb::Mode::Virtex2, &mut test);
        dsp::gen_dsp(&mut ctx, dsp::Mode::Virtex2, &mut test);
        clkbuf::gen_clkbuf(&mut test, &mut ctx, clkbuf::Mode::Virtex2);
        cfg::gen_cfg(&mut test, &mut ctx, cfg_mode);
        if family == "virtex2p" {
            hard::gen_ppc405(&mut test, &mut ctx, false);
        }
        res.push(test);
    }
    res
}

fn get_spartan3_tests(family: &str) -> Vec<Test> {
    let mut res = Vec::new();
    let mut ctx = TestGenCtx::new();
    let (part, ramb_mode, dsp_mode, cfg_mode) = match family {
        "spartan3" => ("xc3s2000-4-fg900", ramb::Mode::Virtex2, dsp::Mode::Virtex2, cfg::Mode::Spartan3),
        "spartan3e" => ("xc3s1600e-4-fg484", ramb::Mode::Virtex2, dsp::Mode::Spartan3E, cfg::Mode::Spartan3E),
        "spartan3a" => ("xc3s1400an-4-fgg676", ramb::Mode::Spartan3A, dsp::Mode::Spartan3E, cfg::Mode::Spartan3A),
        "spartan3adsp" => ("xc3sd1800a-4-fg676", ramb::Mode::Spartan3ADsp, dsp::Mode::Spartan3ADsp, cfg::Mode::Spartan3ADsp),
        _ => unreachable!(),
    };
    for i in 0..10 {
        let mut test = Test::new(&format!("clb{i}"), part);
        clb_lut4::gen_clb(&mut ctx, clb_lut4::Mode::Spartan3, &mut test);
        ramb::gen_ramb(&mut ctx, ramb_mode, &mut test);
        dsp::gen_dsp(&mut ctx, dsp_mode, &mut test);
        clkbuf::gen_clkbuf(&mut test, &mut ctx, clkbuf::Mode::Spartan3);
        cfg::gen_cfg(&mut test, &mut ctx, cfg_mode);
        res.push(test);
    }
    res
}

fn get_virtex4_tests() -> Vec<Test> {
    let mut res = Vec::new();
    let mut ctx = TestGenCtx::new();
    let part = "xc4vfx140-10-ff1517";
    for i in 0..10 {
        let mut test = Test::new(&format!("clb{i}"), part);
        clb_lut4::gen_clb(&mut ctx, clb_lut4::Mode::Virtex4, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("ramb{i}"), part);
        ramb::gen_ramb(&mut ctx, ramb::Mode::Virtex4, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("dsp{i}"), part);
        dsp::gen_dsp(&mut ctx, dsp::Mode::Virtex4, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("hard{i}"), part);
        hard::gen_ppc405(&mut test, &mut ctx, true);
        hard::gen_emac(&mut test, &mut ctx, hard::EmacMode::Virtex4);
        clkbuf::gen_clkbuf(&mut test, &mut ctx, clkbuf::Mode::Virtex4);
        cfg::gen_cfg(&mut test, &mut ctx, cfg::Mode::Virtex4);
        res.push(test);
    }
    res
}

fn get_virtex5_tests() -> Vec<Test> {
    let mut res = Vec::new();
    let mut ctx = TestGenCtx::new();
    let part = "xc5vfx100t-1-ff1738";
    for i in 0..10 {
        let mut test = Test::new(&format!("clb{i}"), part);
        clb_lut6::gen_clb(&mut ctx, clb_lut6::Mode::Virtex5, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("ramb{i}"), part);
        ramb::gen_ramb(&mut ctx, ramb::Mode::Virtex5, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("dsp{i}"), part);
        dsp::gen_dsp(&mut ctx, dsp::Mode::Virtex5, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("hard{i}"), part);
        hard::gen_ppc440(&mut test, &mut ctx);
        hard::gen_emac(&mut test, &mut ctx, hard::EmacMode::Virtex5);
        clkbuf::gen_clkbuf(&mut test, &mut ctx, clkbuf::Mode::Virtex5);
        cfg::gen_cfg(&mut test, &mut ctx, cfg::Mode::Virtex5);
        res.push(test);
    }
    res
}

fn get_virtex6_tests() -> Vec<Test> {
    let mut res = Vec::new();
    let mut ctx = TestGenCtx::new();
    let part = "xc6vlx75t-1-ff784";
    for i in 0..10 {
        let mut test = Test::new(&format!("clb{i}"), part);
        clb_lut6::gen_clb(&mut ctx, clb_lut6::Mode::Virtex6, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("ramb{i}"), part);
        ramb::gen_ramb(&mut ctx, ramb::Mode::Virtex6, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("dsp{i}"), part);
        dsp::gen_dsp(&mut ctx, dsp::Mode::Virtex6, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("hard{i}"), part);
        hard::gen_emac(&mut test, &mut ctx, hard::EmacMode::Virtex6);
        clkbuf::gen_clkbuf(&mut test, &mut ctx, clkbuf::Mode::Virtex6);
        cfg::gen_cfg(&mut test, &mut ctx, cfg::Mode::Virtex6);
        res.push(test);
    }
    res
}

fn get_spartan6_tests() -> Vec<Test> {
    let mut res = Vec::new();
    let mut ctx = TestGenCtx::new();
    let part = "xc6slx150t-2-fgg900";
    for i in 0..10 {
        let mut test = Test::new(&format!("clb{i}"), part);
        clb_lut6::gen_clb(&mut ctx, clb_lut6::Mode::Spartan6, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("ramb{i}"), part);
        ramb::gen_ramb(&mut ctx, ramb::Mode::Spartan6, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("dsp{i}"), part);
        dsp::gen_dsp(&mut ctx, dsp::Mode::Spartan6, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("hard{i}"), part);
        clkbuf::gen_clkbuf(&mut test, &mut ctx, clkbuf::Mode::Spartan6);
        cfg::gen_cfg(&mut test, &mut ctx, cfg::Mode::Spartan6);
        res.push(test);
    }
    res
}

fn get_7series_tests() -> Vec<Test> {
    let mut res = Vec::new();
    let mut ctx = TestGenCtx::new();
    let part = "xc7k70t-1-fbg676";
    for i in 0..10 {
        let mut test = Test::new(&format!("clb{i}"), part);
        clb_lut6::gen_clb(&mut ctx, clb_lut6::Mode::Series7, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("ramb{i}"), part);
        ramb::gen_ramb(&mut ctx, ramb::Mode::Series7, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("dsp{i}"), part);
        dsp::gen_dsp(&mut ctx, dsp::Mode::Series7, &mut test);
        res.push(test);
        let mut test = Test::new(&format!("hard{i}"), part);
        clkbuf::gen_clkbuf(&mut test, &mut ctx, clkbuf::Mode::Series7);
        cfg::gen_cfg(&mut test, &mut ctx, cfg::Mode::Series7);
        res.push(test);
    }
    res
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let tc = Toolchain::from_file(&opt.toolchain)?;
    opt.families.par_iter().for_each(|family| {
        let tests = match &family[..] {
            "spartan2" => get_virtex_tests(family),
            "spartan2e" => get_virtex_tests(family),
            "virtex" => get_virtex_tests(family),
            "virtexe" => get_virtex_tests(family),
            "virtex2" => get_virtex2_tests(family),
            "virtex2p" => get_virtex2_tests(family),
            "spartan3" => get_spartan3_tests(family),
            "spartan3e" => get_spartan3_tests(family),
            "spartan3a" => get_spartan3_tests(family),
            "spartan3adsp" => get_spartan3_tests(family),
            "spartan6" => get_spartan6_tests(),
            "virtex4" => get_virtex4_tests(),
            "virtex5" => get_virtex5_tests(),
            "virtex6" => get_virtex6_tests(),
            "7series" => get_7series_tests(),
            _ => panic!("unknown family {}", family),
        };
        tests.par_iter().for_each(|t| {
            let tn = &t.name;
            println!("Testing {family} {tn}...");
            let v = verilog::emit(&t);
            match run::run(&tc, &t.part, &v) {
                Ok(d) => if !verify::verify(&t, &d, &family) {
                    let mut fv = File::create(format!("fail_{family}_{tn}.v")).unwrap();
                    write!(fv, "{v}").unwrap();
                    let mut fx = File::create(format!("fail_{family}_{tn}.xdl")).unwrap();
                    d.write(&mut fx).unwrap();
                }
                Err(e) => {
                    println!("SYNTH FAIL {:?}", e);
                    let mut fv = File::create(format!("fail_{family}_{tn}.v")).unwrap();
                    write!(fv, "{v}").unwrap();
                }
            }
            println!("Tested {family} {tn}.");
        });
    });
    Ok(())
}
