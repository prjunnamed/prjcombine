use std::collections::HashSet;

use unnamed_entity::EntityPartVec;

use crate::{
    speed::{gen_speed, SpeedData}, tiledb::{check_misc_data, gen_misc_table, gen_tiles, FrameDirection, TileOrientation}, DocgenContext
};

pub fn gen_siliconblue(ctx: &mut DocgenContext) {
    let tile_orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };
    for kind in [
        "ice65l04", "ice65p04", "ice65l08", "ice65l01", "ice40p01", "ice40p03", "ice40p08",
        "ice40r04", "ice40t04", "ice40t05", "ice40t01",
    ] {
        let db = prjcombine_siliconblue::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        gen_tiles(ctx, kind, &db.tiles, |_| tile_orientation);
        let mut speeds = EntityPartVec::new();
        for part in &db.parts {
            for (sname, &speedid) in &part.speeds {
                let speed = &db.speeds[speedid];
                if !speeds.contains_id(speedid) {
                    speeds.insert(
                        speedid,
                        SpeedData {
                            names: vec![],
                            speed,
                        },
                    );
                }
                speeds[speedid]
                    .names
                    .push(format!("{pname}{sname}", pname = part.name));
            }
        }
        gen_speed(ctx, kind, &Vec::from_iter(speeds.into_values()));
        let mut misc_used = HashSet::new();
        if matches!(kind, "ice65l04" | "ice65p04" | "ice65l08") {
            gen_misc_table(
                ctx,
                &db.tiles,
                &mut misc_used,
                kind,
                "iostd-drive",
                &["IOSTD:DRIVE"],
            );
            gen_misc_table(
                ctx,
                &db.tiles,
                &mut misc_used,
                kind,
                "iostd-misc",
                &["IOSTD:IOSTD_MISC"],
            );
        }
        check_misc_data(&db.tiles, kind, &misc_used);
    }
}
