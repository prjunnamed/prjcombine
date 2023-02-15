use std::{collections::HashMap, error::Error};

use prjcombine_ise_dump::partgen::{get_pkgs, PartgenPkg};
use prjcombine_rawdump::PkgPin;
use prjcombine_toolchain::Toolchain;
use prjcombine_xilinx_cpld::device::DeviceKind;
use simple_error::bail;

pub fn get_parts(tc: &Toolchain, kind: DeviceKind) -> Result<Vec<PartgenPkg>, Box<dyn Error>> {
    let mut pkgs = vec![];
    let flist: &[_] = match kind {
        DeviceKind::Xc9500 => &["xc9500"],
        DeviceKind::Xc9500Xl => &["xc9500xl", "xa9500xl"],
        DeviceKind::Xc9500Xv => &["xc9500xv"],
        DeviceKind::Xpla3 => &["xpla3"],
        DeviceKind::Coolrunner2 => &["xbr", "acr2"],
    };
    for f in flist {
        pkgs.extend(get_pkgs(tc, f)?);
    }
    let index: HashMap<_, _> = pkgs
        .iter()
        .map(|p| ((p.device.clone(), p.package.clone()), p.pins.clone()))
        .collect();
    for pkg in &mut pkgs {
        if pkg.pins.is_empty() {
            if !pkg.device.starts_with("xa") {
                bail!("weird case of missing pins {}", pkg.device);
            }
            let dev = format!("xc{}", &pkg.device[2..]);
            pkg.pins = index[&(dev, pkg.package.clone())].clone();
        }
        // the pkg file for this one is wrong.
        if pkg.device == "xc2c128" && pkg.package == "cv100" {
            for pin in &mut pkg.pins {
                let Some(ref pad) = pin.pad else {
                    continue;
                };
                if pad == "PAD33" {
                    assert_eq!(pin.pin, "G4");
                    pin.pin = "K2".to_string();
                }
            }
            pkg.pins.push(PkgPin {
                pad: Some("PAD38".to_string()),
                pin: "G4".to_string(),
                vref_bank: Some(1),
                vcco_bank: Some(1),
                func: "IO".to_string(),
                tracelen_um: None,
                delay_min_fs: None,
                delay_max_fs: None,
            });
        }
    }
    Ok(pkgs)
}
