use jzon::JsonValue;

use crate::db::{
    BelInfo, BelPin, BelSlot, ConnectorClass, ConnectorSlot, ConnectorWire, GroupTestMux,
    GroupTestMuxWire, IntDb, LegacyBel, PinDir, SwitchBox, SwitchBoxItem, TestMux, TestMuxWire,
    TileClass,
};

impl BelPin {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            wires: Vec::from_iter(self.wires.iter().map(|wf| wf.to_string(db, tcls))),
            dir: match self.dir {
                PinDir::Input => "INPUT",
                PinDir::Output => "OUTPUT",
                PinDir::Inout => "INOUT",
            },
        }
    }
}

impl BelInfo {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        match self {
            BelInfo::SwitchBox(sb) => sb.to_json(db, tcls),
            BelInfo::Bel(_bel) => todo!(),
            BelInfo::Legacy(bel) => bel.to_json(db, tcls),
            BelInfo::TestMux(tmux) => tmux.to_json(db, tcls),
            BelInfo::GroupTestMux(tmux) => tmux.to_json(db, tcls),
        }
    }
}

impl SwitchBox {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            kind: "switchbox",
            items: Vec::from_iter(self.items.iter().map(|item| item.to_json(db, tcls))),
        }
    }
}

impl SwitchBoxItem {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        match self {
            SwitchBoxItem::Mux(mux) => jzon::object! {
                kind: "mux",
                dst: mux.dst.to_string(db, tcls),
                src: Vec::from_iter(mux.src.keys().map(|w| w.to_string(db, tcls))),
            },
            SwitchBoxItem::ProgBuf(buf) => jzon::object! {
                kind: "progbuf",
                dst: buf.dst.to_string(db, tcls),
                src: buf.src.to_string(db, tcls),
            },
            SwitchBoxItem::PermaBuf(buf) => jzon::object! {
                kind: "permabuf",
                dst: buf.dst.to_string(db, tcls),
                src: buf.src.to_string(db, tcls),
            },
            SwitchBoxItem::Pass(pass) => jzon::object! {
                kind: "pass",
                dst: pass.dst.to_string(db, tcls),
                src: pass.src.to_string(db, tcls),
            },
            SwitchBoxItem::BiPass(pass) => jzon::object! {
                kind: "pass",
                wires: [
                    pass.a.to_string(db, tcls),
                    pass.b.to_string(db, tcls),
                ],
            },
            SwitchBoxItem::ProgInv(inv) => jzon::object! {
                kind: "proginv",
                dst: inv.dst.to_string(db, tcls),
                src: inv.src.to_string(db, tcls),
            },
            SwitchBoxItem::ProgDelay(delay) => jzon::object! {
                kind: "progdelay",
                dst: delay.dst.to_string(db, tcls),
                src: delay.src.to_string(db, tcls),
                num_steps: delay.steps.len(),
            },
        }
    }
}

impl LegacyBel {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            kind: "bel",
            pins: jzon::object::Object::from_iter(self.pins.iter().map(|(pname, pin)| (pname.as_str(), pin.to_json(db, tcls)))),
        }
    }
}

impl TestMux {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            kind: "test_mux",
            wires: jzon::object::Object::from_iter(
                self.wires.iter().map(|(dst, tmwire)| (dst.to_string(db, tcls), tmwire.to_json(db, tcls)))
            )
        }
    }
}

impl TestMuxWire {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            primary_src: self.primary_src.to_string(db, tcls),
            test_src: Vec::from_iter(self.test_src.iter().map(|wire| wire.to_string(db, tcls))),
        }
    }
}

impl GroupTestMux {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            kind: "group_test_mux",
            num_groups: self.num_groups,
            wires: jzon::object::Object::from_iter(
                self.wires.iter().map(|(dst, tmwire)| (dst.to_string(db, tcls), tmwire.to_json(db, tcls)))
            )
        }
    }
}

impl GroupTestMuxWire {
    pub fn to_json(&self, db: &IntDb, tcls: &TileClass) -> JsonValue {
        jzon::object! {
            primary_src: self.primary_src.to_string(db, tcls),
            test_src: Vec::from_iter(self.test_src.iter().map(|wire|
                if let Some(wire) = wire {
                    JsonValue::from(wire.to_string(db, tcls))
                } else {
                    JsonValue::Null
                }
            )),
        }
    }
}

impl TileClass {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        jzon::object! {
            slot: db.tile_slots[self.slot].as_str(),
            cells: self.cells.len(),
            bels: jzon::object::Object::from_iter(self.bels.iter().map(|(slot, bel)| (
                db.bel_slots.key(slot).as_str(),
                bel.to_json(db, self),
            ))),
        }
    }
}

impl BelSlot {
    pub fn to_json(self, db: &IntDb) -> JsonValue {
        jzon::object! {
            tile_slot: db.tile_slots[self.tile_slot].as_str(),
        }
    }
}

impl ConnectorSlot {
    pub fn to_json(self, db: &IntDb) -> JsonValue {
        jzon::object! {
            opposite: db.conn_slots.key(self.opposite).as_str(),
        }
    }
}

impl ConnectorWire {
    pub fn to_json(self, db: &IntDb) -> JsonValue {
        match self {
            ConnectorWire::BlackHole => jzon::object! {
                kind: "BLACKHOLE",
            },
            ConnectorWire::Reflect(wf) => jzon::object! {
                kind: "REFLECT",
                wire: db.wires.key(wf).as_str(),
            },
            ConnectorWire::Pass(wf) => jzon::object! {
                kind: "PASS",
                wire: db.wires.key(wf).as_str(),
            },
        }
    }
}

impl ConnectorClass {
    pub fn to_json(&self, db: &IntDb) -> JsonValue {
        jzon::object! {
            slot: db.conn_slots.key(self.slot).as_str(),
            wires: jzon::object::Object::from_iter(self.wires.iter().map(|(wire, ti)|
                (db.wires.key(wire).to_string(), ti.to_json(db))
            ))
        }
    }
}

impl From<&IntDb> for JsonValue {
    fn from(db: &IntDb) -> Self {
        jzon::object! {
            wires: Vec::from_iter(db.wires.iter().map(|(_, name, wire)| {
                jzon::object! {
                    name: name.as_str(),
                    kind: wire.to_string(db),
                }
            })),
            region_slots: Vec::from_iter(db.region_slots.values().map(|name| name.as_str())),
            tile_slots: Vec::from_iter(db.tile_slots.values().map(|name| name.as_str())),
            bel_slots: jzon::object::Object::from_iter(db.bel_slots.iter().map(|(_, name, bslot)| {
                (name.as_str(), bslot.to_json(db))
            })),
            tile_classes: jzon::object::Object::from_iter(db.tile_classes.iter().map(|(_, name, tcls)| {
                (name.as_str(), tcls.to_json(db))
            })),
            conn_slots: jzon::object::Object::from_iter(db.conn_slots.iter().map(|(_, name, cslot)| {
                (name.as_str(), cslot.to_json(db))
            })),
            conn_classes: jzon::object::Object::from_iter(db.conn_classes.iter().map(|(_, name, ccls)| {
                (name.as_str(), ccls.to_json(db))
            })),
        }
    }
}
