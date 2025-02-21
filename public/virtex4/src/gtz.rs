use std::collections::BTreeMap;

use prjcombine_interconnect::db::{Dir, PinDir};
use serde::{Deserialize, Serialize};
use serde_json::json;
use unnamed_entity::{EntityMap, entity_id};

entity_id! {
    pub id GtzBelId u16;
    pub id GtzIntColId u16, delta;
    pub id GtzIntRowId u16, delta;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GtzBel {
    pub side: Dir,
    pub pins: BTreeMap<String, GtzIntPin>,
    pub clk_pins: BTreeMap<String, GtzClkPin>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GtzIntPin {
    pub dir: PinDir,
    pub col: GtzIntColId,
    pub row: GtzIntRowId,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GtzClkPin {
    pub dir: PinDir,
    pub idx: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GtzDb {
    pub gtz: EntityMap<GtzBelId, String, GtzBel>,
}

impl GtzDb {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Map::from_iter(self.gtz.iter().map(|(_, name, gtz)| {
            (
                name.clone(),
                json!({
                    "side": gtz.side.to_string(),
                    "pins": serde_json::Map::from_iter(gtz.pins.iter().map(|(pname, pin)|
                        (pname.clone(), json!({
                            "dir": match pin.dir {
                                PinDir::Input => "INPUT",
                                PinDir::Output => "OUTPUT",
                                PinDir::Inout => unreachable!(),
                            },
                            "col": pin.col,
                            "row": pin.row,
                        }))
                    )),
                    "clk_pins": serde_json::Map::from_iter(gtz.clk_pins.iter().map(|(pname, pin)|
                        (pname.clone(), json!({
                            "dir": match pin.dir {
                                PinDir::Input => "INPUT",
                                PinDir::Output => "OUTPUT",
                                PinDir::Inout => unreachable!(),
                            },
                            "idx": pin.idx,
                        }))
                    )),
                }),
            )
        }))
        .into()
    }
}

impl std::fmt::Display for GtzDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (_, name, gtz) in &self.gtz {
            writeln!(f, "GTZ {name} [{side}]:", side = gtz.side)?;
            for (pname, pin) in &gtz.pins {
                writeln!(
                    f,
                    "\tPIN {pname:20}: {dir:6} INT {col}.{row}",
                    dir = match pin.dir {
                        PinDir::Input => "INPUT",
                        PinDir::Output => "OUTPUT",
                        PinDir::Inout => unreachable!(),
                    },
                    col = pin.col,
                    row = pin.row
                )?;
            }
            for (pname, pin) in &gtz.clk_pins {
                writeln!(
                    f,
                    "\tPIN {pname:20}: {dir:6} GCLK{idx}",
                    dir = match pin.dir {
                        PinDir::Input => "INPUT",
                        PinDir::Output => "OUTPUT",
                        PinDir::Inout => unreachable!(),
                    },
                    idx = pin.idx
                )?;
            }
        }
        Ok(())
    }
}
