use std::collections::HashSet;

use unnamed_entity::EntityPartVec;

use crate::{
    DocgenContext,
    bsdata::{FrameDirection, TileOrientation, check_misc_data, gen_bstiles, gen_misc_table},
    interconnect::gen_intdb,
    speed::{SpeedData, gen_speed},
};

pub fn gen_siliconblue(ctx: &mut DocgenContext) {
    let tile_orientation = TileOrientation {
        frame_direction: FrameDirection::Horizontal,
        flip_frame: false,
        flip_bit: false,
    };
    let db = prjcombine_siliconblue::db::Database::from_file(
        ctx.ctx.root.join("../databases/siliconblue.zstd"),
    )
    .unwrap();
    gen_intdb(ctx, "siliconblue", &db.int);
    gen_bstiles(ctx, "siliconblue", &db.bsdata, |_| tile_orientation);
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
    gen_speed(ctx, "siliconblue", &Vec::from_iter(speeds.into_values()));
    let mut misc_used = HashSet::new();
    gen_misc_table(
        ctx,
        &db.bsdata,
        &mut misc_used,
        "siliconblue",
        "iostd-drive",
        &["IOSTD:DRIVE"],
    );
    gen_misc_table(
        ctx,
        &db.bsdata,
        &mut misc_used,
        "siliconblue",
        "iostd-misc",
        &["IOSTD:IOSTD_MISC"],
    );
    check_misc_data(&db.bsdata, "siliconblue", &misc_used);
}
