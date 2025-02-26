use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use bitvec::prelude::*;
use prjcombine_interconnect::{
    db::PinDir,
    dir::Dir,
    grid::{ColId, DieId, IntWire, RowId},
};
use prjcombine_siliconblue::{chip::ChipKind, expanded::ExpandedDevice};
use prjcombine_types::tiledb::{TileDb, TileItemKind};
use unnamed_entity::{EntityId, EntityPartVec};

use crate::{
    parts::Part,
    prims::{Primitive, PropKind},
    run::{Design, InstPin, Instance, RawLoc, run},
    xlat::{GenericNet, xlat_wire},
};

#[derive(Debug, Clone)]
pub struct SiteInfo {
    pub loc: RawLoc,
    pub pads: BTreeMap<String, (RawLoc, String)>,
    pub out_wires: BTreeMap<InstPin, (u32, u32, String)>,
    pub in_wires: BTreeMap<InstPin, (u32, u32, String)>,
    pub fabout_wires: BTreeMap<InstPin, (u32, u32)>,
    pub global_nets: BTreeMap<InstPin, u32>,
}

fn find_sites(
    start_num: usize,
    mut f: impl FnMut(usize) -> Option<Vec<SiteInfo>>,
) -> Vec<SiteInfo> {
    let mut best = vec![];
    let mut low = 0;
    let mut high = start_num;
    while let Some(res) = f(high) {
        best = res;
        low = high;
        high *= 2;
    }
    while high - low > 1 {
        let num = (high + low) / 2;
        if let Some(res) = f(num) {
            best = res;
            low = num;
        } else {
            high = num;
        }
    }
    best
}

pub fn find_sites_plb(sbt: &Path, part: &Part) -> Vec<SiteInfo> {
    find_sites(512, |num| {
        if part.name == "iCE40MX16K" && num >= 1913 {
            // ??????? this hangs sbtplacer.
            return None;
        }
        let mut design = Design {
            kind: part.kind,
            device: part.name,
            package: part.packages[0],
            speed: part.speeds[0],
            temp: part.temps[0],
            insts: Default::default(),
            keep_tmp: false,
            opts: vec![],
        };
        let mut inst = Instance::new("SB_IO");
        inst.top_port("PACKAGE_PIN");
        let mut chain_site = design.insts.push(inst);
        let mut chain_pin = InstPin::Simple("D_IN_0".into());
        for _ in 0..num {
            let mut inst = Instance::new("SB_LUT4");
            inst.prop("LUT_INIT", "16'h0000");
            let lut = design.insts.push(inst);

            let mut inst = Instance::new("SB_DFF");
            inst.connect("D", lut, InstPin::Simple("O".into()));
            inst.connect("C", chain_site, chain_pin);
            chain_site = design.insts.push(inst);
            chain_pin = InstPin::Simple("Q".into());
        }
        let mut inst = Instance::new("SB_IO");
        inst.top_port("PACKAGE_PIN");
        inst.connect("D_OUT_0", chain_site, chain_pin);
        design.insts.push(inst);

        match run(sbt, &design) {
            Err(_) => None,
            Ok(res) => {
                let mut locs = vec![];
                for (iid, loc) in &res.loc_map {
                    if design.insts[iid].kind != "SB_DFF" {
                        continue;
                    }
                    assert!(!loc.is_io);
                    let mut info = SiteInfo {
                        loc: loc.loc,
                        pads: Default::default(),
                        out_wires: Default::default(),
                        in_wires: Default::default(),
                        global_nets: Default::default(),
                        fabout_wires: Default::default(),
                    };
                    let paths = &res.routes[&(iid, InstPin::Simple("Q".into()))];
                    assert_eq!(paths.len(), 1);
                    let path = &paths[0];
                    info.out_wires
                        .insert(InstPin::Simple("Q".into()), path[0].clone());
                    locs.push(info);
                }
                locs.sort_by_key(|loc| loc.loc);
                assert_eq!(locs.len(), num);
                Some(locs)
            }
        }
    })
}

pub fn find_sites_misc(
    sbt: &Path,
    prims: &BTreeMap<&'static str, Primitive>,
    part: &Part,
    pkg: &'static str,
    kind: &str,
) -> Vec<SiteInfo> {
    let start = match kind {
        "SB_IO" => 32,
        "SB_GB_IO" => 8,
        "SB_IO_DLY" => 8,
        "SB_IO_OD" => 4,
        "SB_IO_I3C" => 2,
        "SB_RAM4K" | "SB_RAM40_4K" | "SB_RAM40_16K" => 32,
        "SB_MAC16" => 8,
        "SB_SPRAM256KA" => 8,
        "SB_GB" => 8,
        "SB_SPI" | "SB_I2C" => 2,
        _ => 1,
    };
    find_sites(start, |num| {
        if kind == "SB_WARMBOOT" && num > 1 {
            return None;
        }
        let mut design = Design {
            kind: part.kind,
            device: part.name,
            package: pkg,
            speed: part.speeds[0],
            temp: part.temps[0],
            insts: Default::default(),
            keep_tmp: false,
            opts: vec![],
        };
        let prim = &prims[kind];
        let mut gbs = vec![];
        let mut trace_ins = EntityPartVec::new();
        let mut trace_outs = EntityPartVec::new();
        if kind.starts_with("SB_RAM40") {
            for _ in 0..4 {
                let mut lut = Instance::new("SB_LUT4");
                lut.prop("LUT_INIT", "16'h0000");
                let lut = design.insts.push(lut);
                let mut gb = Instance::new("SB_GB");
                gb.connect(
                    "USER_SIGNAL_TO_GLOBAL_BUFFER",
                    lut,
                    InstPin::Simple("O".into()),
                );
                let gb = design.insts.push(gb);
                gbs.push(gb);
            }
        }
        let mut outps = vec![];
        for idx in 0..num {
            let mut inst = Instance::new(kind);
            let mut cur_trace_ins = vec![];
            let mut cur_trace_outs = vec![];
            for (&pname, pin) in &prim.pins {
                if pin.dir == PinDir::Input && !pin.is_pad {
                    if (kind.starts_with("SB_IO") || kind == "SB_GB_IO")
                        && matches!(
                            pname,
                            "OUTPUT_ENABLE"
                                | "CLOCK_ENABLE"
                                | "INPUT_CLK"
                                | "OUTPUT_CLK"
                                | "LATCH_INPUT_VALUE"
                                | "OUTPUTENABLE"
                                | "CLOCKENABLE"
                                | "INPUTCLK"
                                | "OUTPUTCLK"
                                | "LATCHINPUTVALUE"
                        )
                    {
                        continue;
                    }
                    if kind == "SB_SPRAM256KA"
                        && matches!(pname, "RDMARGIN" | "RDMARGINEN" | "TEST")
                    {
                        continue;
                    }
                    if kind == "SB_IO_DLY" && matches!(pname, "SDI" | "SCLK" | "C_R_SEL") {
                        continue;
                    }
                    if kind == "SB_MAC16" && part.kind == ChipKind::Ice40MX && pname == "CI" {
                        continue;
                    }
                    if kind == "SB_MAC16" && matches!(pname, "ACCUMCI" | "SIGNEXTIN") {
                        continue;
                    }
                    if (kind == "SB_IR_DRV" && pname == "IRPU")
                        || (kind == "SB_RGB_DRV" && pname == "RGBPU")
                    {
                        let mut lut = Instance::new("SB_LUT4");
                        lut.prop("LUT_INIT", "16'h0000");
                        let lut = design.insts.push(lut);

                        let mut drv = Instance::new("SB_LED_DRV_CUR");
                        drv.connect("EN", lut, InstPin::Simple("O".into()));
                        let drv = design.insts.push(drv);

                        inst.connect(pname, drv, InstPin::Simple("LEDPU".into()));

                        cur_trace_ins.push((lut, InstPin::Simple(pname.into())));

                        continue;
                    }
                    if kind.starts_with("SB_RAM40")
                        && matches!(pname, "RCLK" | "RE" | "WCLK" | "WE")
                    {
                        let idx = ["RCLK", "RE", "WCLK", "WE"]
                            .iter()
                            .position(|&n| n == pname)
                            .unwrap();
                        inst.connect(
                            pname,
                            gbs[idx],
                            InstPin::Simple("GLOBAL_BUFFER_OUTPUT".into()),
                        );
                        continue;
                    }
                    match pin.len {
                        Some(n) => {
                            for i in 0..n {
                                let mut lut = Instance::new("SB_LUT4");
                                lut.prop("LUT_INIT", "16'h0000");
                                let lut = design.insts.push(lut);
                                inst.connect_idx(pname, i, lut, InstPin::Simple("O".into()));
                                cur_trace_ins.push((lut, InstPin::Indexed(pname.into(), i)));
                            }
                        }
                        None => {
                            let mut lut = Instance::new("SB_LUT4");
                            lut.prop("LUT_INIT", "16'h0000");
                            let lut = design.insts.push(lut);
                            inst.connect(pname, lut, InstPin::Simple("O".into()));
                            cur_trace_ins.push((lut, InstPin::Simple(pname.into())));
                        }
                    }
                }
                if pin.is_pad {
                    inst.top_port(pname);
                }
            }
            for (&pname, pval) in &prim.props {
                if pname.ends_with("_CURRENT") {
                    let PropKind::BitvecBinStr(len) = *pval else {
                        unreachable!()
                    };
                    inst.prop_bin_str(pname, &bitvec![1; len]);
                    continue;
                }
                if pname == "BUS_ADDR74" {
                    match kind {
                        "SB_I2C" => {
                            inst.prop(pname, ["0b0001", "0b0011"][idx % 2]);
                        }
                        "SB_SPI" => {
                            inst.prop(pname, ["0b0000", "0b0010"][idx % 2]);
                        }
                        _ => unreachable!(),
                    }
                    continue;
                }
                if pname == "I2C_SLAVE_INIT_ADDR" || pname == "I2C_SLAVE_ADDR" {
                    inst.prop(pname, ["0b1111100001", "0b1111100010"][idx % 2]);
                    continue;
                }
                if kind == "SB_MAC16" && pname == "MODE_8x8" {
                    inst.prop_bin(pname, bits![1]);
                    continue;
                }
                if kind == "SB_MAC16"
                    && (pname == "BOTOUTPUT_SELECT" || pname == "TOPOUTPUT_SELECT")
                {
                    inst.prop_bin(pname, bits![0, 1]);
                    continue;
                }

                match pval {
                    PropKind::String(vals) => {
                        inst.prop(pname, vals[0]);
                    }
                    PropKind::BitvecHex(l) => {
                        inst.prop_bin(pname, &bitvec![0; *l]);
                    }
                    PropKind::BitvecBin(l) => {
                        inst.prop_bin(pname, &bitvec![0; *l]);
                    }
                    PropKind::BitvecBinStr(l) => {
                        inst.prop_bin_str(pname, &bitvec![0; *l]);
                    }
                }
            }
            let inst = design.insts.push(inst);
            for (&pname, pin) in &prim.pins {
                if pin.dir == PinDir::Output && !pin.is_pad {
                    if kind == "SB_IO_DLY" && matches!(pname, "SDO") {
                        continue;
                    }
                    if kind == "SB_MAC16" && part.kind == ChipKind::Ice40MX && pname == "CO" {
                        continue;
                    }
                    if (kind.starts_with("SB_IO") || kind == "SB_GB_IO")
                        && matches!(pname, "D_IN_0" | "D_IN_1" | "DIN0" | "DIN1")
                    {
                        continue;
                    }
                    if kind == "SB_MAC16" && matches!(pname, "ACCUMCO" | "SIGNEXTOUT") {
                        continue;
                    }
                    if (matches!(
                        kind,
                        "SB_GB" | "SB_GB_IO" | "SB_HSOSC" | "SB_LSOSC" | "SB_HFOSC" | "SB_LFOSC"
                    )) || pname.starts_with("PLLOUTGLOBAL")
                        || (kind.starts_with("SB_PLL") && pname == "SDO")
                    {
                        let pin = InstPin::Simple(pname.into());
                        cur_trace_outs.push(pin.clone());
                        let mut lut = Instance::new("SB_LUT4");
                        lut.connect("I0", inst, pin);
                        lut.prop("LUT_INIT", "16'haaaa");
                        let lut = design.insts.push(lut);
                        outps.push((lut, InstPin::Simple("O".into())));
                        continue;
                    }
                    match pin.len {
                        Some(n) => {
                            for i in 0..n {
                                let pin = InstPin::Indexed(pname.to_string(), i);
                                outps.push((inst, pin.clone()));
                                cur_trace_outs.push(pin);
                            }
                        }
                        None => {
                            let pin = InstPin::Simple(pname.to_string());
                            outps.push((inst, pin.clone()));
                            cur_trace_outs.push(pin);
                        }
                    }
                }
            }
            trace_ins.insert(inst, cur_trace_ins);
            trace_outs.insert(inst, cur_trace_outs);
        }
        while outps.len() > 1 {
            let mut lut = Instance::new("SB_LUT4");
            lut.prop("LUT_INIT", "16'h6996");
            for pin in ["I0", "I1", "I2", "I3"] {
                if let Some((sinst, spin)) = outps.pop() {
                    lut.connect(pin, sinst, spin);
                }
            }
            let lut = design.insts.push(lut);
            let mut ff = Instance::new("SB_DFF");
            ff.connect("D", lut, InstPin::Simple("O".into()));
            let ff = design.insts.push(ff);
            outps.push((ff, InstPin::Simple("Q".into())));
        }
        if !outps.is_empty() {
            let (sinst, spin) = outps.pop().unwrap();
            let mut io = Instance::new("SB_IO");
            io.top_port("PACKAGE_PIN");
            io.connect("D_OUT_0", sinst, spin);
            design.insts.push(io);
        }
        match run(sbt, &design) {
            Err(err) => {
                if !err.stdout.contains("Error: Design Feasibility Failed")
                    && !err
                        .stdout
                        .contains("Unable to fit the design into the selected device/package")
                    && !err
                        .stdout
                        .contains("Unable to fit the design into the selected device.")
                    && !err.stdout.contains(
                        "Please change the family/device/package and re-import your design.",
                    )
                    && !err
                        .stdout
                        .contains("Feasibility check for IO Placement failed")
                    && !err.stdout.contains("Error during global Buffer placement")
                    && !err.stdout.contains("placement is infeasible for the design")
                    && !err.stdout.contains("Tool unable to complete IOPlacement for the design")
                    && !err.stdout.contains("does not support SB_IO_DS. Instead, use SB_IO with IO_STANDARD property as SB_LVDS_INPUT")
                    && !err.stdout.contains(
                        "Please choose correct package and re-import your design.",
                    )
                {
                    println!("FAIL FOR {kind}:");
                    println!("{}", err.stdout);
                }
                None
            }
            Ok(res) => {
                let mut locs = vec![];
                for (iid, loc) in &res.loc_map {
                    if design.insts[iid].kind != kind {
                        continue;
                    }
                    let mut info = SiteInfo {
                        loc: loc.loc,
                        pads: Default::default(),
                        out_wires: Default::default(),
                        in_wires: Default::default(),
                        global_nets: Default::default(),
                        fabout_wires: Default::default(),
                    };
                    if kind.starts_with("SB_IO") || kind == "SB_GB_IO" || kind == "SB_GB" {
                        assert!(loc.is_io);
                    } else {
                        assert!(!loc.is_io);
                    }
                    for (&pname, pin) in &prim.pins {
                        if pin.is_pad {
                            let ipin = InstPin::Simple(pname.to_string());
                            if let Some(io_loc) = res.io_map.get(&(iid, ipin)) {
                                info.pads
                                    .insert(pname.to_string(), (io_loc.loc, io_loc.pin.clone()));
                            }
                        }
                    }
                    for pin in &trace_outs[iid] {
                        let paths = &res.routes[&(iid, pin.clone())];
                        assert_eq!(paths.len(), 1);
                        let path = &paths[0];
                        info.out_wires.insert(pin.clone(), path[0].clone());
                        if !kind.ends_with("OSC") {
                            for (_, _, wname) in path {
                                if let Some(idx) = wname.strip_prefix("glb_netwk_") {
                                    let idx = idx.parse().unwrap();
                                    info.global_nets.insert(pin.clone(), idx);
                                }
                            }
                        }
                    }
                    for &(lut, ref pin) in &trace_ins[iid] {
                        if kind == "SB_IR500_DRV" {
                            continue;
                        }
                        if kind == "SB_IR_IP" && *pin == InstPin::Simple("RST".into()) {
                            continue;
                        }
                        if (kind == "SB_LEDD_IP" || kind == "SB_LEDDA_IP")
                            && *pin == InstPin::Simple("LEDDRST".into())
                        {
                            continue;
                        }
                        let paths = &res.routes[&(lut, InstPin::Simple("O".into()))];
                        assert_eq!(paths.len(), 1);
                        let path = &paths[0];
                        info.in_wires
                            .insert(pin.clone(), path.last().unwrap().clone());
                        if matches!(kind, "SB_GB" | "SB_WARMBOOT" | "SB_LSOSC" | "SB_HSOSC")
                            || (kind.starts_with("SB_PLL") && design.kind != ChipKind::Ice40T01)
                        {
                            let mut coord = None;
                            for &(x, y, ref name) in path {
                                if name.starts_with("lc_trk_g") {
                                    assert!(coord.is_none());
                                    coord = Some((x, y));
                                }
                            }
                            info.fabout_wires.insert(pin.clone(), coord.unwrap());
                        }
                    }
                    locs.push(info);
                }
                locs.sort_by_key(|loc| loc.loc);
                assert_eq!(locs.len(), num);
                Some(locs)
            }
        }
    })
}

pub fn find_io_latch_locs(
    sbt: &Path,
    part: &Part,
    pkg: &'static str,
    pkg_pins: &HashMap<Dir, &str>,
) -> HashMap<Dir, (u32, u32)> {
    let mut design = Design {
        kind: part.kind,
        device: part.name,
        package: pkg,
        speed: part.speeds[0],
        temp: part.temps[0],
        insts: Default::default(),
        keep_tmp: false,
        opts: vec![],
    };
    let mut trace_ins = vec![];
    for (&edge, &pkg_pin) in pkg_pins {
        let mut inst = Instance::new("SB_IO");
        inst.io
            .insert(InstPin::Simple("PACKAGE_PIN".into()), pkg_pin.to_string());
        inst.top_port("PACKAGE_PIN");

        let mut lut = Instance::new("SB_LUT4");
        lut.prop("LUT_INIT", "16'h0000");
        let lut = design.insts.push(lut);
        inst.connect("LATCH_INPUT_VALUE", lut, InstPin::Simple("O".into()));
        trace_ins.push((lut, edge));

        inst.prop("PIN_TYPE", "6'b000011");

        design.insts.push(inst);
    }
    match run(sbt, &design) {
        Err(err) => {
            panic!("FAIL TO LOCATE IO LATCH: {err:#?}");
        }
        Ok(res) => {
            let mut result = HashMap::new();
            for (lut, edge) in trace_ins {
                let paths = &res.routes[&(lut, InstPin::Simple("O".into()))];
                assert_eq!(paths.len(), 1);
                let path = &paths[0];
                let mut coord = None;
                for &(x, y, ref name) in path {
                    if name.starts_with("lc_trk_g") {
                        assert!(coord.is_none());
                        coord = Some((x, y));
                    }
                }
                result.insert(edge, coord.unwrap());
            }
            result
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BelPins {
    pub ins: BTreeMap<String, IntWire>,
    pub outs: BTreeMap<String, Vec<IntWire>>,
    pub wire_names: BTreeMap<(u32, u32, String), IntWire>,
}

#[allow(clippy::too_many_arguments)]
pub fn find_bel_pins(
    sbt: &Path,
    prims: &BTreeMap<&'static str, Primitive>,
    part: &Part,
    edev: &ExpandedDevice,
    tiledb: Option<&TileDb>,
    pkg: &'static str,
    kind: &str,
    site: &SiteInfo,
) -> BelPins {
    let mut result = BelPins::default();
    if edev.chip.kind.has_actual_lrio() {
        for (k, &v) in &site.fabout_wires {
            let iw = (
                DieId::from_idx(0),
                (ColId::from_idx(v.0 as usize), RowId::from_idx(v.1 as usize)),
                edev.egrid.db.get_wire("IMUX.IO.EXTRA"),
            );
            match k {
                InstPin::Simple(pin) => {
                    if pin == "LATCHINPUTVALUE" {
                        continue;
                    }
                    result.ins.insert(pin.clone(), iw);
                }
                InstPin::Indexed(pin, index) => {
                    result.ins.insert(format!("{pin}_{index}"), iw);
                }
            }
            result.wire_names.insert(site.in_wires[k].clone(), iw);
        }
        if kind.starts_with("SB_PLL") {
            let row = if site.loc.y == 0 {
                edev.chip.row_bio()
            } else {
                edev.chip.row_tio()
            };
            for (pin, col) in [("LOCK", edev.chip.col_lio()), ("SDO", edev.chip.col_rio())] {
                let iws = Vec::from_iter((0..8).map(|idx| {
                    (
                        DieId::from_idx(0),
                        (col, row),
                        edev.egrid.db.get_wire(&format!("OUT.LC{idx}")),
                    )
                }));
                result
                    .wire_names
                    .insert(site.out_wires[&InstPin::Simple(pin.into())].clone(), iws[0]);
                result.outs.insert(pin.into(), iws);
            }
        }
    } else {
        let mut design = Design {
            kind: part.kind,
            device: part.name,
            package: pkg,
            speed: part.speeds[0],
            temp: part.temps[0],
            insts: Default::default(),
            keep_tmp: false,
            opts: vec![],
        };
        let prim = &prims[kind];
        let mut inst = Instance::new(kind);
        if kind == "SB_IO_I3C" {
            inst.io.insert(
                InstPin::Simple("PACKAGE_PIN".into()),
                site.pads["PACKAGE_PIN"].1.clone(),
            );
        } else {
            inst.loc = Some(site.loc);
        }
        let mut trace_ins = BTreeMap::new();

        for (&pname, pin) in &prim.pins {
            if pin.dir == PinDir::Input && !pin.is_pad {
                if (kind.starts_with("SB_IO") || kind == "SB_GB_IO")
                    && matches!(
                        pname,
                        "OUTPUT_ENABLE"
                            | "CLOCK_ENABLE"
                            | "INPUT_CLK"
                            | "OUTPUT_CLK"
                            | "LATCH_INPUT_VALUE"
                            | "OUTPUTENABLE"
                            | "CLOCKENABLE"
                            | "INPUTCLK"
                            | "OUTPUTCLK"
                            | "LATCHINPUTVALUE"
                    )
                {
                    continue;
                }
                if kind == "SB_MAC16" && matches!(pname, "ACCUMCI" | "SIGNEXTIN") {
                    continue;
                }
                if (kind == "SB_IR_DRV" && pname == "IRPU")
                    || (kind == "SB_RGB_DRV" && pname == "RGBPU")
                {
                    let mut drv = Instance::new("SB_LED_DRV_CUR");

                    let mut lut = Instance::new("SB_LUT4");
                    lut.prop("LUT_INIT", "16'h0000");
                    let lut = design.insts.push(lut);
                    trace_ins.insert(
                        (lut, InstPin::Simple("O".into())),
                        InstPin::Simple("LED_DRV_CUR__EN".into()),
                    );
                    drv.connect("EN", lut, InstPin::Simple("O".into()));

                    for i in 0..10 {
                        let mut lut = Instance::new("SB_LUT4");
                        lut.prop("LUT_INIT", "16'h0000");
                        let lut = design.insts.push(lut);
                        trace_ins.insert(
                            (lut, InstPin::Simple("O".into())),
                            InstPin::Simple(format!("LED_DRV_CUR__TRIM{i}")),
                        );
                        drv.connect(&format!("TRIM{i}"), lut, InstPin::Simple("O".into()));
                    }

                    let drv = design.insts.push(drv);

                    inst.connect(pname, drv, InstPin::Simple("LEDPU".into()));

                    continue;
                }
                match pin.len {
                    Some(n) => {
                        for i in 0..n {
                            let mut lut = Instance::new("SB_LUT4");
                            lut.prop("LUT_INIT", "16'h0000");
                            let lut = design.insts.push(lut);
                            inst.connect_idx(pname, i, lut, InstPin::Simple("O".into()));
                            trace_ins.insert(
                                (lut, InstPin::Simple("O".into())),
                                InstPin::Indexed(pname.into(), i),
                            );
                        }
                    }
                    None => {
                        let mut lut = Instance::new("SB_LUT4");
                        lut.prop("LUT_INIT", "16'h0000");
                        let lut = design.insts.push(lut);
                        inst.connect(pname, lut, InstPin::Simple("O".into()));
                        trace_ins.insert(
                            (lut, InstPin::Simple("O".into())),
                            InstPin::Simple(pname.into()),
                        );
                    }
                }
            }
            if pin.is_pad {
                inst.top_port(pname);
            }
        }
        for (&pname, pval) in &prim.props {
            if pname.ends_with("_CURRENT") {
                let PropKind::BitvecBinStr(len) = *pval else {
                    unreachable!()
                };
                inst.prop_bin_str(pname, &bitvec![1; len]);
                continue;
            }
            if pname == "BUS_ADDR74" {
                let idx = if site.loc.x == 0 { 0 } else { 1 };
                match kind {
                    "SB_I2C" => {
                        inst.prop(pname, ["0b0001", "0b0011"][idx % 2]);
                    }
                    "SB_SPI" => {
                        inst.prop(pname, ["0b0000", "0b0010"][idx % 2]);
                    }
                    _ => unreachable!(),
                }
                continue;
            }
            if pname == "I2C_SLAVE_INIT_ADDR" || pname == "I2C_SLAVE_ADDR" {
                let idx = if site.loc.x == 0 { 0 } else { 1 };
                inst.prop(pname, ["0b1111100001", "0b1111100010"][idx % 2]);
                continue;
            }
            if kind == "SB_MAC16" && pname == "MODE_8x8" {
                inst.prop_bin(pname, bits![1]);
                continue;
            }
            if kind == "SB_MAC16" && (pname == "BOTOUTPUT_SELECT" || pname == "TOPOUTPUT_SELECT") {
                inst.prop_bin(pname, bits![0, 1]);
                continue;
            }

            match pval {
                PropKind::String(vals) => {
                    inst.prop(pname, vals[0]);
                }
                PropKind::BitvecHex(l) => {
                    inst.prop_bin(pname, &bitvec![0; *l]);
                }
                PropKind::BitvecBin(l) => {
                    inst.prop_bin(pname, &bitvec![0; *l]);
                }
                PropKind::BitvecBinStr(l) => {
                    inst.prop_bin_str(pname, &bitvec![0; *l]);
                }
            }
        }

        if kind.ends_with("OSC") {
            inst.prop("ROUTE_THROUGH_FABRIC", "1");
        }

        let inst = design.insts.push(inst);
        let mut outps = vec![];
        for (&pname, pin) in &prim.pins {
            if pin.dir == PinDir::Output && !pin.is_pad {
                if kind == "SB_MAC16" && part.kind == ChipKind::Ice40MX && pname == "CO" {
                    continue;
                }
                if (kind.starts_with("SB_IO") || kind == "SB_GB_IO")
                    && matches!(pname, "D_IN_0" | "D_IN_1" | "DIN0" | "DIN1")
                {
                    continue;
                }
                if kind == "SB_MAC16" && matches!(pname, "ACCUMCO" | "SIGNEXTOUT") {
                    continue;
                }
                if (matches!(
                    kind,
                    "SB_GB" | "SB_GB_IO" | "SB_HSOSC" | "SB_LSOSC" | "SB_HFOSC" | "SB_LFOSC"
                )) || pname.starts_with("PLLOUTGLOBAL")
                    || (kind.starts_with("SB_PLL") && pname == "SDO")
                {
                    let pin = InstPin::Simple(pname.into());
                    let mut lut = Instance::new("SB_LUT4");
                    lut.connect("I0", inst, pin);
                    lut.prop("LUT_INIT", "16'haaaa");
                    let lut = design.insts.push(lut);
                    outps.push((lut, InstPin::Simple("O".into())));
                    continue;
                }
                match pin.len {
                    Some(n) => {
                        for i in 0..n {
                            let pin = InstPin::Indexed(pname.to_string(), i);
                            outps.push((inst, pin.clone()));
                        }
                    }
                    None => {
                        let pin = InstPin::Simple(pname.to_string());
                        outps.push((inst, pin.clone()));
                    }
                }
            }
        }

        while outps.len() > 1 {
            let mut lut = Instance::new("SB_LUT4");
            lut.prop("LUT_INIT", "16'h6996");
            for pin in ["I0", "I1", "I2", "I3"] {
                if let Some((sinst, spin)) = outps.pop() {
                    lut.connect(pin, sinst, spin);
                }
            }
            let lut = design.insts.push(lut);
            let mut ff = Instance::new("SB_DFF");
            ff.connect("D", lut, InstPin::Simple("O".into()));
            let ff = design.insts.push(ff);
            outps.push((ff, InstPin::Simple("Q".into())));
        }
        if !outps.is_empty() {
            let (sinst, spin) = outps.pop().unwrap();
            let mut io = Instance::new("SB_IO");
            io.top_port("PACKAGE_PIN");
            io.connect("D_OUT_0", sinst, spin);
            design.insts.push(io);
        }
        let res = run(sbt, &design).unwrap();
        let mut iwmap_in = BTreeMap::new();
        let mut iwmap_out = BTreeMap::new();
        let mut wnmap: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (src, paths) in res.routes {
            if src.0 == inst {
                for path in paths {
                    for (x, y, wn) in path {
                        match xlat_wire(edev, x, y, &wn) {
                            GenericNet::Int(iw) => {
                                iwmap_out.insert(iw, src.1.clone());
                            }
                            GenericNet::Unknown => {
                                wnmap.entry(src.1.clone()).or_default().push((x, y, wn));
                            }
                            _ => (),
                        }
                    }
                }
            } else if let Some(pin) = trace_ins.get(&src) {
                for path in paths {
                    for (x, y, wn) in path {
                        match xlat_wire(edev, x, y, &wn) {
                            GenericNet::Int(iw) => {
                                iwmap_in.insert(iw, pin.clone());
                            }
                            GenericNet::Unknown => {
                                wnmap.entry(pin.clone()).or_default().push((x, y, wn));
                            }
                            _ => (),
                        }
                    }
                }
            }
        }

        let tiledb = tiledb.unwrap();
        for col in edev.chip.columns() {
            for row in edev.chip.rows() {
                let tile_kind = if row == edev.chip.row_bio() {
                    if col == edev.chip.col_lio() || col == edev.chip.col_rio() {
                        continue;
                    }
                    "IO.B"
                } else if row == edev.chip.row_tio() {
                    if col == edev.chip.col_lio() || col == edev.chip.col_rio() {
                        continue;
                    }
                    "IO.T"
                } else if col == edev.chip.col_lio() && edev.chip.kind.has_lrio() {
                    "IO.L"
                } else if col == edev.chip.col_rio() && edev.chip.kind.has_lrio() {
                    "IO.R"
                } else if edev.chip.cols_bram.contains(&col) {
                    "INT.BRAM"
                } else {
                    "PLB"
                };
                let tile = &tiledb.tiles[tile_kind];
                let btile = edev.btile_main(col, row);
                for (name, item) in &tile.items {
                    let (wtn, wfn) = if let Some(wtn) = name.strip_prefix("INT:MUX.") {
                        let TileItemKind::Enum { values } = &item.kind else {
                            unreachable!()
                        };
                        let mut bv = bitvec![];
                        for &bit in &item.bits {
                            let bit = btile.xlat_pos_fwd((bit.frame, bit.bit));
                            let val = res.bitstream.get(bit);
                            bv.push(val);
                        }
                        let wfn = values.iter().find(|&(_, val)| *val == bv).unwrap().0;
                        if wfn == "NONE" {
                            continue;
                        }
                        (wtn, wfn.as_str())
                    } else if let Some(rest) = name.strip_prefix("INT:BUF.") {
                        let TileItemKind::BitVec { invert } = &item.kind else {
                            unreachable!()
                        };
                        let Some((wtn, _)) = rest.split_once(".OUT.LC") else {
                            continue;
                        };
                        let wfn = rest.strip_prefix(wtn).unwrap().strip_prefix('.').unwrap();
                        let bit = item.bits[0];
                        let bit = btile.xlat_pos_fwd((bit.frame, bit.bit));
                        let bit = res.bitstream.get(bit) ^ invert[0];
                        if !bit {
                            continue;
                        }
                        (wtn, wfn)
                    } else {
                        continue;
                    };
                    if wtn.starts_with("IMUX") {
                        let wf = (DieId::from_idx(0), (col, row), edev.egrid.db.get_wire(wfn));
                        let wf = edev.egrid.resolve_wire(wf).unwrap();
                        if let Some(pin) = iwmap_in.get(&wf) {
                            let wt = (DieId::from_idx(0), (col, row), edev.egrid.db.get_wire(wtn));
                            if let Some(wnames) = wnmap.get(pin) {
                                for wn in wnames {
                                    result.wire_names.insert(wn.clone(), wt);
                                }
                            }
                            let pin = match pin {
                                InstPin::Simple(pin) => pin.clone(),
                                InstPin::Indexed(pin, index) => format!("{pin}_{index}"),
                            };
                            result.ins.insert(pin, wt);
                        }
                    }
                    if wfn.starts_with("OUT") {
                        let wt = (DieId::from_idx(0), (col, row), edev.egrid.db.get_wire(wtn));
                        let wt = edev.egrid.resolve_wire(wt).unwrap();
                        if let Some(pin) = iwmap_out.get(&wt) {
                            let wf = (DieId::from_idx(0), (col, row), edev.egrid.db.get_wire(wfn));
                            let wf = edev.egrid.resolve_wire(wf).unwrap();
                            let is_lr =
                                wf.1.0 == edev.chip.col_lio() || wf.1.0 == edev.chip.col_rio();
                            let is_bt =
                                wf.1.1 == edev.chip.row_bio() || wf.1.1 == edev.chip.row_tio();
                            let wfs = if is_lr && is_bt {
                                Vec::from_iter((0..8).map(|idx| {
                                    (wf.0, wf.1, edev.egrid.db.get_wire(&format!("OUT.LC{idx}")))
                                }))
                            } else if (is_lr && edev.chip.kind.has_lrio()) || is_bt {
                                let wfn = edev.egrid.db.wires.key(wf.2);
                                let idx: usize =
                                    wfn.strip_prefix("OUT.LC").unwrap().parse().unwrap();
                                let idx = idx & 3;
                                Vec::from_iter([idx, idx + 4].map(|idx| {
                                    (wf.0, wf.1, edev.egrid.db.get_wire(&format!("OUT.LC{idx}")))
                                }))
                            } else {
                                vec![wf]
                            };
                            if let Some(wnames) = wnmap.get(pin) {
                                for wn in wnames {
                                    result.wire_names.insert(wn.clone(), wfs[0]);
                                }
                            }
                            let pin = match pin {
                                InstPin::Simple(pin) => pin.clone(),
                                InstPin::Indexed(pin, index) => format!("{pin}_{index}"),
                            };
                            result.outs.insert(pin, wfs);
                        }
                    }
                }
            }
        }
    }
    result
}
