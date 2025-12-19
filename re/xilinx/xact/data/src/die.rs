use std::{collections::BTreeMap, path::Path};

use bytes::{Buf, Bytes};
use ndarray::Array2;
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec, entity_id};
use prjcombine_interconnect::dir::{Dir, DirMap};

entity_id! {
    pub id PrimDefId u16;
    pub id PrimId u16;
    pub id PinId u16;
    pub id BoxDefId u16;
    pub id BoxId u16;
}

#[derive(Debug)]
pub struct Die {
    pub version_a: u16,
    pub version_b: u16,
    pub kind: u16,
    pub grwidth: u32,
    pub grheight: u32,
    pub unk3: u32,
    pub unk4: u16,
    pub is_tiled: bool,
    pub unk8: u16,
    pub unk9: u16,
    pub unk10: u16,
    pub unk11: u16,
    pub unk12: u16,
    pub unk13: u16,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub matrix: Option<Array2<u16>>,
    pub matrix_cells_bwd: DirMap<[u8; 0x100]>,
    pub matrix_cells_fwd: DirMap<[u8; 0x100]>,
    pub matrix_cells_flags: [u8; 0x100],
    pub die: String,
    pub prog: String,
    pub version: String,
    pub time: String,
    pub date: String,
    pub boxdefs: EntityVec<BoxDefId, BoxDef>,
    pub boxes: EntityVec<BoxId, BoxInst>,
    pub primdefs: EntityVec<PrimDefId, PrimDef>,
    pub prims: EntityVec<PrimId, PrimInst>,
    pub tbufs: EntityPartVec<PrimId, Tbuf>,
    pub tiledefs: BTreeMap<String, TileDef>,
    pub tiles: BTreeMap<(usize, usize), String>,
    pub newtiledefs: BTreeMap<String, NewTileDef>,
    pub newtiles: BTreeMap<(usize, usize), NewTile>,
    pub newcols: Vec<NewColumn>,
    pub newrows: Vec<NewRow>,
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub grpos: u32,
    pub unk0: u16,
}

#[derive(Debug)]
pub struct Row {
    pub name: String,
    pub grpos: u32,
    pub unk0: u16,
}

#[derive(Debug)]
pub struct Rect {
    pub unk0: i16,
    pub unk1: i16,
    pub unk2: i16,
    pub unk3: i16,
}

#[derive(Debug)]
pub struct BoxInst {
    pub name: String,
    pub lname: String,
    pub bx: u16,
    pub by: u16,
    pub boxdef: BoxDefId,
}

#[derive(Debug)]
pub struct BoxDef {
    pub name: String,
    pub pins: Vec<BoxDefPin>,
    pub rect: Rect,
    pub unk0: i16,
    pub unk1: i16,
    pub unk2: i16,
    pub unk3: i16,
    pub unk4: i16,
}

#[derive(Debug)]
pub struct PrimDef {
    pub name: String,
    pub grwidth: u16,
    pub grheight: u16,
    pub pins: EntityVec<PinId, PinDef>,
    pub kind: u8,
    pub unk0: u16,
    pub unk1: u16,
}

#[derive(Debug)]
pub struct PinDef {
    pub name: String,
    pub grx: i16,
    pub gry: i16,
    pub mode: char,
    pub side: Dir,
    pub unk0: u16,
    pub unk1: u8,
    pub kind: u16,
    pub unk2: u16,
}

#[derive(Debug)]
pub struct PrimInst {
    pub name_a: String,
    pub name_b: String,
    pub name_i: String,
    pub primdef: PrimDefId,
    pub grx: u32,
    pub gry: u32,
    pub padid: u16,
    pub unk0: u16,
    pub col: String,
    pub row: String,
    pub unk1: u16,
    pub unk2: u16,
    pub pins: EntityVec<PinId, PinInst>,
}

#[derive(Debug)]
pub struct PinInst {
    pub x: usize,
    pub y: usize,
    pub unk0: u16,
    pub unk1: u16,
}

#[derive(Debug)]
pub struct BoxDefPin {
    pub mask: Vec<bool>,
    pub side: Dir,
    pub dx: i16,
    pub dy: i16,
}

#[derive(Debug)]
pub struct Tbuf {
    pub out_x: u16,
    pub out_y: u16,
    pub ins: Vec<TbufInput>,
}

#[derive(Debug)]
pub struct TbufInput {
    pub in_x: u16,
    pub in_y: u16,
    pub pip_x: u16,
    pub pip_y: u16,
}

#[derive(Debug)]
pub struct TileDef {
    pub matrix: Array2<u16>,
}

#[derive(Debug)]
pub struct NewTileDef {
    pub unk0: u16,
    pub unk1: u16,
    pub name_alt: String,
    pub matrix: Array2<NewTileCell>,
    pub segs: Vec<NewTileSeg>,
    pub pips: Vec<NewTilePip>,
    pub pipxref: Vec<usize>,
}

#[derive(Debug)]
pub struct NewTileCell {
    pub u: Option<usize>,
    pub r: Option<usize>,
    pub d: Option<usize>,
    pub l: Option<usize>,
}

#[derive(Debug)]
pub struct NewTileSeg {
    pub name: String,
    pub unk0: u16,
    pub unk1: u16,
    pub unk2: u16,
    pub pin_xy: Option<(u16, u16)>,
    pub x: u16,
    pub y: u16,
    pub unk3: u16,
}

#[derive(Debug)]
pub struct NewTilePip {
    pub seg_dst: usize,
    pub seg_src: usize,
    pub x: u16,
    pub y: u16,
    pub box_pins: Option<(usize, usize)>,
    pub flags: u16,
    pub cls: u16,
    pub unk0: u32,
}

#[derive(Debug)]
pub struct NewTile {
    pub kind: String,
    pub prims: Vec<PrimId>,
    pub boxes: Vec<BoxId>,
    pub name: String,
    pub unk: String,
    pub col: String,
    pub row: String,
}

#[derive(Debug)]
pub struct NewColumn {
    pub name: String,
    pub x: u16,
}

#[derive(Debug)]
pub struct NewRow {
    pub name: String,
    pub y: u16,
}

fn chksum(stream: &mut Bytes) {
    assert_eq!(stream.get_u8(), 0x76);
    stream.get_u16();
    stream.get_u16();
}

fn get_string(stream: &mut Bytes) -> String {
    let sz = stream.get_u16();
    let mut data = vec![];
    for _ in 0..sz {
        data.push(stream.get_u8());
    }
    assert_eq!(data.pop(), Some(0));
    String::from_utf8(data).unwrap()
}

fn get_rect(stream: &mut Bytes) -> Rect {
    let unk0 = stream.get_i16();
    let unk1 = stream.get_i16();
    let unk2 = stream.get_i16();
    let unk3 = stream.get_i16();
    Rect {
        unk0,
        unk1,
        unk2,
        unk3,
    }
}

impl Die {
    pub fn parse(xact: &Path, dev: &str) -> Die {
        let path = xact.join(format!("xact/data/{dev}.die"));
        let data = std::fs::read(path).unwrap();
        let mut stream = Bytes::from(data);
        assert_eq!(stream.get_u8(), 0x61);
        let version_a = stream.get_u16();
        let version_b = stream.get_u16();
        let kind = stream.get_u16();
        let num_columns: usize = stream.get_u16().into();
        let num_rows: usize = stream.get_u16().into();
        let grwidth: u32;
        let grheight: u32;
        let unk3: u32;
        if version_a < 46 {
            grwidth = stream.get_u16().into();
            grheight = stream.get_u16().into();
            unk3 = stream.get_u16().into();
        } else {
            grwidth = stream.get_u32();
            grheight = stream.get_u32();
            unk3 = stream.get_u32();
        }
        let unk4 = stream.get_u16();
        let num_pins: usize = stream.get_u16().into();
        let num_prims: usize = stream.get_u16().into();
        let is_tiled = match stream.get_u16() {
            0 => false,
            1 => true,
            x => panic!("umm wtf is bool {x}"),
        };
        let num_boxes: usize = stream.get_u16().into();
        let num_primdefs: usize = stream.get_u16().into();
        let num_pindefs: usize = stream.get_u16().into();
        let num_boxdefs: usize = stream.get_u16().into();
        let num_tbufs: usize = stream.get_u16().into();
        let unk8 = stream.get_u16();
        let unk9 = stream.get_u16();
        let unk10 = stream.get_u16();
        let unk11 = stream.get_u16();
        let unk12 = stream.get_u16();
        let unk13 = stream.get_u16();
        assert_eq!(stream.get_u8(), 0);
        chksum(&mut stream);
        let mut columns = vec![];
        for _ in 0..num_columns {
            assert_eq!(stream.get_u8(), 0x63);
            let name = get_string(&mut stream);
            let grpos: u32 = if version_a < 46 {
                stream.get_u16().into()
            } else {
                stream.get_u32()
            };
            let unk0 = stream.get_u16();
            columns.push(Column { name, grpos, unk0 });
            assert_eq!(stream.get_u8(), 0);
        }
        chksum(&mut stream);
        let mut rows = vec![];
        for _ in 0..num_rows {
            assert_eq!(stream.get_u8(), 0x64);
            let name = get_string(&mut stream);
            let grpos: u32 = if version_a < 46 {
                stream.get_u16().into()
            } else {
                stream.get_u32()
            };
            let unk0 = stream.get_u16();
            rows.push(Row { name, grpos, unk0 });
            assert_eq!(stream.get_u8(), 0);
        }
        chksum(&mut stream);
        let matrix = if !is_tiled {
            let mut matrix = Array2::from_elem((num_columns, num_rows), 0);
            for row in 0..num_rows {
                assert_eq!(stream.get_u8(), 0x65);
                for col in 0..num_columns {
                    matrix[(col, row)] = stream.get_u16();
                }
                assert_eq!(stream.get_u8(), 0);
                chksum(&mut stream);
            }
            Some(matrix)
        } else {
            None
        };
        chksum(&mut stream);
        assert_eq!(stream.get_u8(), 0x6d);
        let matrix_cells_bwd_n: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        let matrix_cells_bwd_e: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        let matrix_cells_bwd_s: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        let matrix_cells_bwd_w: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        let matrix_cells_bwd = DirMap::from_fn(|dir| match dir {
            Dir::N => matrix_cells_bwd_n,
            Dir::E => matrix_cells_bwd_e,
            Dir::S => matrix_cells_bwd_s,
            Dir::W => matrix_cells_bwd_w,
        });
        assert_eq!(stream.get_u8(), 0);
        chksum(&mut stream);
        assert_eq!(stream.get_u8(), 0x6c);
        let matrix_cells_fwd_n: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        let matrix_cells_fwd_e: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        let matrix_cells_fwd_s: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        let matrix_cells_fwd_w: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        let matrix_cells_fwd = DirMap::from_fn(|dir| match dir {
            Dir::N => matrix_cells_fwd_n,
            Dir::E => matrix_cells_fwd_e,
            Dir::S => matrix_cells_fwd_s,
            Dir::W => matrix_cells_fwd_w,
        });
        assert_eq!(stream.get_u8(), 0);
        chksum(&mut stream);
        assert_eq!(stream.get_u8(), 0x6e);
        let matrix_cells_flags: [u8; 0x100] = core::array::from_fn(|_| stream.get_u8());
        assert_eq!(stream.get_u8(), 0);
        chksum(&mut stream);

        let die = get_string(&mut stream);
        let prog = get_string(&mut stream);
        let version = get_string(&mut stream);
        let time = get_string(&mut stream);
        let date = get_string(&mut stream);
        chksum(&mut stream);

        let num_tiles: usize;
        let num_tile_cols: usize;
        let num_tile_rows: usize;
        if version_a < 46 {
            num_tiles = 0;
            num_tile_cols = 0;
            num_tile_rows = 0;
        } else {
            num_tiles = stream.get_u16().into();
            num_tile_cols = stream.get_u16().into();
            num_tile_rows = stream.get_u16().into();
        }

        let mut boxdefs = EntityPartVec::new();
        let mut boxes = EntityPartVec::new();
        let mut primdefs = EntityPartVec::new();
        let mut prims = EntityPartVec::new();
        let mut prim_pins = EntityPartVec::new();
        let mut primdef_pins = EntityPartVec::new();
        let mut tbufs = EntityPartVec::new();
        let mut pd_num_pins = EntityPartVec::new();
        let mut tiledefs = BTreeMap::new();
        let mut newtiledefs = BTreeMap::new();
        let mut tiles = BTreeMap::new();
        let mut newtiles = BTreeMap::new();
        let mut newcols = vec![];
        let mut newrows = vec![];
        let mut last_newtile = None;
        loop {
            let code = stream.get_u8();
            match code {
                0x66 => {
                    // box
                    let name = get_string(&mut stream);
                    let lname = get_string(&mut stream);
                    let bx = stream.get_u16();
                    let by = stream.get_u16();
                    let id = BoxId::from_idx(stream.get_u16().into());
                    let boxdef = BoxDefId::from_idx(stream.get_u16().into());
                    assert_eq!(stream.get_u8(), 0);
                    let boxx = BoxInst {
                        name,
                        lname,
                        bx,
                        by,
                        boxdef,
                    };
                    assert!(boxes.insert(id, boxx).is_none());
                }
                0x67 => {
                    // prim def
                    let name = get_string(&mut stream);
                    let id = PrimDefId::from_idx(stream.get_u16().into());
                    let grwidth = stream.get_u16();
                    let grheight = stream.get_u16();
                    let num_pins: usize = stream.get_u16().into();
                    let kind = stream.get_u8();
                    let unk0;
                    let unk1;
                    if version_a < 46 {
                        unk0 = 0;
                        unk1 = 0;
                    } else {
                        unk0 = stream.get_u16();
                        unk1 = stream.get_u16();
                    }
                    assert_eq!(stream.get_u8(), 0);
                    let primdef = PrimDef {
                        name,
                        grwidth,
                        grheight,
                        pins: EntityVec::new(),
                        kind,
                        unk0,
                        unk1,
                    };
                    pd_num_pins.insert(id, num_pins);
                    primdef_pins.insert(id, EntityPartVec::new());
                    assert!(primdefs.insert(id, primdef).is_none());
                }
                0x68 => {
                    // pin def
                    let pdid = PrimDefId::from_idx(stream.get_u16().into());
                    let name = get_string(&mut stream);
                    let grx = stream.get_i16();
                    let gry = stream.get_i16();
                    let mode: char = stream.get_u8().into();
                    let side = match stream.get_u16() {
                        0 => Dir::N,
                        1 => Dir::E,
                        2 => Dir::S,
                        3 => Dir::W,
                        s => panic!("weird side {s}"),
                    };
                    let pdid_dup = PrimDefId::from_idx(stream.get_u16().into());
                    assert_eq!(pdid, pdid_dup);
                    let unk0 = stream.get_u16();
                    let unk1 = stream.get_u8();
                    let kind = stream.get_u16();
                    let idx = PinId::from_idx(stream.get_u16().into());
                    let unk2 = stream.get_u16();
                    assert_eq!(stream.get_u8(), 0);
                    let pindef = PinDef {
                        name,
                        grx,
                        gry,
                        mode,
                        side,
                        unk0,
                        unk1,
                        kind,
                        unk2,
                    };
                    assert!(primdef_pins[pdid].insert(idx, pindef).is_none());
                }
                0x69 => {
                    // prim
                    let name_a = get_string(&mut stream);
                    let name_b = get_string(&mut stream);
                    let name_i = get_string(&mut stream);
                    let primdef = PrimDefId::from_idx(stream.get_u16().into());
                    let grx: u32;
                    let gry: u32;
                    if version_a < 46 {
                        grx = stream.get_u16().into();
                        gry = stream.get_u16().into();
                    } else {
                        grx = stream.get_u32();
                        gry = stream.get_u32();
                    }
                    let padid = stream.get_u16();
                    let unk0 = stream.get_u16();
                    let id = PrimId::from_idx(stream.get_u16().into());
                    let col = get_string(&mut stream);
                    let row = get_string(&mut stream);
                    let unk1;
                    let unk2;
                    if version_a < 46 {
                        unk1 = 0;
                        unk2 = 0;
                    } else {
                        unk1 = stream.get_u16();
                        unk2 = stream.get_u16();
                    }
                    assert_eq!(stream.get_u8(), 0);
                    let prim = PrimInst {
                        name_a,
                        name_b,
                        name_i,
                        primdef,
                        grx,
                        gry,
                        padid,
                        unk0,
                        col,
                        row,
                        unk1,
                        unk2,
                        pins: EntityVec::new(),
                    };
                    assert!(prims.insert(id, prim).is_none());
                    prim_pins.insert(id, EntityPartVec::new());
                }
                0x6f => {
                    // box def
                    let name = get_string(&mut stream);
                    let num: usize = stream.get_u16().into();
                    let id = BoxDefId::from_idx(stream.get_u16().into());
                    let mut ends = vec![];
                    let nbytes = num.div_ceil(8);
                    for _ in 0..num {
                        let mut mask = vec![];
                        for b in 0..nbytes {
                            let byte = stream.get_u8();
                            for i in 0..8 {
                                let col = b * 8 + i;
                                if col < num {
                                    mask.push((byte >> (i ^ 7) & 1) != 0);
                                }
                            }
                        }
                        let side = match stream.get_u16() {
                            0 => Dir::N,
                            1 => Dir::E,
                            2 => Dir::S,
                            3 => Dir::W,
                            s => panic!("weird side {s}"),
                        };
                        let dx = stream.get_i16();
                        let dy = stream.get_i16();
                        ends.push(BoxDefPin { mask, side, dx, dy })
                    }
                    let rect = get_rect(&mut stream);
                    let unk0 = stream.get_i16();
                    let unk1 = stream.get_i16();
                    let unk2 = stream.get_i16();
                    let unk3 = stream.get_i16();
                    let unk4 = stream.get_i16();
                    assert_eq!(stream.get_u8(), 0);
                    let boxdef = BoxDef {
                        name,
                        pins: ends,
                        rect,
                        unk0,
                        unk1,
                        unk2,
                        unk3,
                        unk4,
                    };
                    assert!(boxdefs.insert(id, boxdef).is_none());
                }
                0x72 => {
                    // pin inst
                    let id = PinId::from_idx(stream.get_u16().into());
                    let prim = PrimId::from_idx(stream.get_u16().into());
                    let x = usize::from(stream.get_u16());
                    let y = usize::from(stream.get_u16());
                    let unk0 = stream.get_u16();
                    let unk1 = stream.get_u16();
                    assert_eq!(stream.get_u8(), 0);
                    let pin = PinInst { x, y, unk0, unk1 };
                    assert!(prim_pins[prim].insert(id, pin).is_none());
                }
                0x73 => {
                    // tbuf
                    let id = PrimId::from_idx(stream.get_u16().into());
                    let num_ins = stream.get_u16();
                    let out_x = stream.get_u16();
                    let out_y = stream.get_u16();
                    let mut ins = vec![];
                    for _ in 0..num_ins {
                        let in_x = stream.get_u16();
                        let in_y = stream.get_u16();
                        let pip_x = stream.get_u16();
                        let pip_y = stream.get_u16();
                        ins.push(TbufInput {
                            in_x,
                            in_y,
                            pip_x,
                            pip_y,
                        });
                    }
                    assert_eq!(stream.get_u8(), 0);
                    let tbuf = Tbuf { out_x, out_y, ins };
                    tbufs.insert(id, tbuf);
                }
                0x74 => {
                    // tile def
                    let name = get_string(&mut stream).to_ascii_lowercase();
                    let height = usize::from(stream.get_u16() + 1);
                    let width = usize::from(stream.get_u16() + 1);
                    let x = usize::from(stream.get_u16());
                    let y = usize::from(stream.get_u16()) - (height - 1);
                    tiles.insert((x, y), name.clone());
                    let mut matrix = Array2::from_elem((width, height), 0);
                    for row in 0..height {
                        for col in 0..width {
                            matrix[(width - 1 - col, row)] = stream.get_u16();
                        }
                    }
                    let tiledef = TileDef { matrix };
                    tiledefs.insert(name, tiledef);
                    assert_eq!(stream.get_u8(), 0);
                }
                0x75 => {
                    // tile use
                    let name = get_string(&mut stream).to_ascii_lowercase();
                    let td = &tiledefs[&name];
                    let x = usize::from(stream.get_u16());
                    let y = usize::from(stream.get_u16()) - (td.matrix.dim().1 - 1);
                    tiles.insert((x, y), name);
                    assert_eq!(stream.get_u8(), 0);
                }
                0x76 => {
                    // chksum
                    stream.get_u16();
                    stream.get_u16();
                }
                0x6a => {
                    // eof
                    assert!(!stream.has_remaining());
                    break;
                }
                0x41 => {
                    // new tile def
                    let unk0 = stream.get_u16();
                    let unk1 = stream.get_u16();
                    let height: usize = (stream.get_u16() + 1).into();
                    let width: usize = (stream.get_u16() + 1).into();
                    let name_alt = get_string(&mut stream);
                    let name = get_string(&mut stream);
                    let mut matrix =
                        Array2::from_shape_simple_fn((width, height), || NewTileCell {
                            u: None,
                            r: None,
                            d: None,
                            l: None,
                        });
                    for y in 0..height {
                        assert_eq!(stream.get_u8(), 0x45);
                        for x in 0..width {
                            let cell = &mut matrix[(x, y)];
                            let mask = stream.get_u8();
                            if (mask & 1) != 0 {
                                cell.u = Some(stream.get_u16().into());
                            }
                            if (mask & 2) != 0 {
                                cell.r = Some(stream.get_u16().into());
                            }
                            if (mask & 4) != 0 {
                                cell.d = Some(stream.get_u16().into());
                            }
                            if (mask & 8) != 0 {
                                cell.l = Some(stream.get_u16().into());
                            }
                        }
                        assert_eq!(stream.get_u8(), 0x46);
                    }
                    let num_segs: usize = stream.get_u32().try_into().unwrap();
                    let mut segs = vec![];
                    for i in 0..num_segs {
                        let name = get_string(&mut stream);
                        let unk0 = stream.get_u16();
                        let unk1 = stream.get_u16();
                        let idx = stream.get_u32();
                        assert_eq!(i, usize::try_from(idx).unwrap());
                        let unk2 = stream.get_u16();
                        let pin_x = stream.get_u16();
                        let pin_y = stream.get_u16();
                        let pin_xy = if pin_x == 0xffff || pin_y == 0xffff {
                            assert_eq!(pin_x, 0xffff);
                            assert_eq!(pin_y, 0xffff);
                            None
                        } else {
                            Some((pin_x, pin_y))
                        };
                        let x = stream.get_u16();
                        let y = stream.get_u16();
                        let unk3 = stream.get_u16();
                        segs.push(NewTileSeg {
                            name,
                            unk0,
                            unk1,
                            unk2,
                            pin_xy,
                            x,
                            y,
                            unk3,
                        });
                    }
                    let num_pips: usize = stream.get_u32().try_into().unwrap();
                    let mut pips = vec![];
                    for i in 0..num_pips {
                        let idx = stream.get_u32();
                        assert_eq!(i, usize::try_from(idx).unwrap());
                        let seg_dst: usize = stream.get_u32().try_into().unwrap();
                        let seg_src: usize = stream.get_u32().try_into().unwrap();
                        let x = stream.get_u16();
                        let y = stream.get_u16();
                        let box_pin_a: usize = stream.get_u16().into();
                        let box_pin_b: usize = stream.get_u16().into();
                        let box_pins = if box_pin_a == 0 && box_pin_b == 0 {
                            None
                        } else {
                            Some((box_pin_a, box_pin_b))
                        };
                        let flags = stream.get_u16();
                        let cls = stream.get_u16();
                        let unk0 = stream.get_u32();
                        pips.push(NewTilePip {
                            seg_dst,
                            seg_src,
                            x,
                            y,
                            box_pins,
                            flags,
                            cls,
                            unk0,
                        });
                    }
                    let mut pipxref = vec![];
                    for _ in 0..(num_pips * 2) {
                        let idx: usize = stream.get_u32().try_into().unwrap();
                        pipxref.push(idx);
                    }
                    assert_eq!(stream.get_u8(), 0);
                    last_newtile = Some(name.clone());
                    newtiledefs.insert(
                        name,
                        NewTileDef {
                            unk0,
                            unk1,
                            name_alt,
                            matrix,
                            segs,
                            pips,
                            pipxref,
                        },
                    );
                }
                0x42 => {
                    // new tile
                    let ntd = &newtiledefs[last_newtile.as_ref().unwrap()];
                    let x = usize::try_from(stream.get_u32()).unwrap();
                    let y = usize::try_from(stream.get_u32()).unwrap() - (ntd.matrix.dim().1 - 1);
                    let num: usize = (stream.get_u16() - 1).into();
                    let mut prims = vec![];
                    for _ in 0..num {
                        let prim = PrimId::from_idx(stream.get_u16().into());
                        prims.push(prim);
                    }
                    let num: usize = (stream.get_u16() - 1).into();
                    let mut boxes = vec![];
                    for _ in 0..num {
                        let boxx = BoxId::from_idx(stream.get_u16().into());
                        boxes.push(boxx);
                    }
                    let name = get_string(&mut stream);
                    let unk = get_string(&mut stream);
                    let col = get_string(&mut stream);
                    let row = get_string(&mut stream);
                    assert_eq!(stream.get_u8(), 0);
                    newtiles.insert(
                        (x, y),
                        NewTile {
                            kind: last_newtile.clone().unwrap(),
                            prims,
                            boxes,
                            name,
                            unk,
                            col,
                            row,
                        },
                    );
                }
                0x43 => {
                    let name = get_string(&mut stream);
                    let x: u16 = stream.get_u32().try_into().unwrap();
                    newcols.push(NewColumn { name, x });
                    assert_eq!(stream.get_u8(), 0);
                }
                0x44 => {
                    let name = get_string(&mut stream);
                    let y: u16 = stream.get_u32().try_into().unwrap();
                    newrows.push(NewRow { name, y });
                    assert_eq!(stream.get_u8(), 0);
                }
                _ => {
                    panic!("UMMMM CODE {code:02x}");
                }
            }
        }
        for (k, v) in &mut primdefs {
            let pins = primdef_pins.remove(k).unwrap().into_full();
            assert_eq!(pins.len(), pd_num_pins[k]);
            v.pins = pins;
        }
        for (k, v) in &mut prims {
            let pins = prim_pins.remove(k).unwrap().into_full();
            assert_eq!(pins.len(), pd_num_pins[v.primdef]);
            v.pins = pins;
        }
        let boxes = boxes.into_full();
        let boxdefs = boxdefs.into_full();
        let prims = prims.into_full();
        let primdefs = primdefs.into_full();
        assert_eq!(num_tbufs, tbufs.iter().count());
        assert_eq!(num_boxes, boxes.len());
        assert_eq!(num_prims, prims.len());
        assert_eq!(num_primdefs, primdefs.len());
        assert_eq!(num_boxdefs, boxdefs.len());
        assert_eq!(
            num_pins,
            prims.values().map(|prim| prim.pins.len()).sum::<usize>()
        );
        assert_eq!(
            num_pindefs,
            primdefs
                .values()
                .map(|primdef| primdef.pins.len())
                .sum::<usize>()
        );
        assert_eq!(num_tile_cols * num_tile_rows, num_tiles);
        if is_tiled {
            assert_eq!(tiles.len(), num_tiles);
        }
        Die {
            version_a,
            version_b,
            kind,
            grwidth,
            grheight,
            unk3,
            unk4,
            is_tiled,
            unk8,
            unk9,
            unk10,
            unk11,
            unk12,
            unk13,
            columns,
            rows,
            matrix,
            matrix_cells_bwd,
            matrix_cells_fwd,
            matrix_cells_flags,
            die,
            prog,
            version,
            time,
            date,
            boxdefs,
            boxes,
            primdefs,
            prims,
            tbufs,
            tiledefs,
            tiles,
            newtiledefs,
            newtiles,
            newcols,
            newrows,
        }
    }

    pub fn box_pin(&self, box_id: BoxId, pin: usize) -> (usize, usize, Dir) {
        let boxx = &self.boxes[box_id];
        let boxdef = &self.boxdefs[boxx.boxdef];
        let pin = &boxdef.pins[pin];
        let x = usize::from(boxx.bx)
            .checked_add_signed(isize::from(pin.dx))
            .unwrap();
        let y = usize::from(boxx.by)
            .checked_add_signed(isize::from(pin.dy))
            .unwrap();
        (x, y, pin.side)
    }

    pub fn make_unified_matrix(&self) -> Array2<u16> {
        if let Some(ref matrix) = self.matrix {
            matrix.clone()
        } else {
            let mut res = Array2::from_elem((self.columns.len(), self.rows.len()), 0xffff);
            for (&(bx, by), kind) in &self.tiles {
                let td = &self.tiledefs[kind];
                for dx in 0..td.matrix.dim().0 {
                    for dy in 0..td.matrix.dim().1 {
                        let tx = bx + dx;
                        let ty = by + dy;
                        assert_eq!(res[(tx, ty)], 0xffff);
                        res[(tx, ty)] = td.matrix[(dx, dy)];
                    }
                }
            }
            for col in 0..self.columns.len() {
                for row in 0..self.rows.len() {
                    assert_ne!(res[(col, row)], 0xffff);
                }
            }
            res
        }
    }
}
