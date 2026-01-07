pub const ROWS: [char; 20] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'R', 'T', 'U', 'V', 'W',
    'Y',
];

pub fn ball(x: u8, y: u8) -> String {
    if y >= 20 {
        let l1 = (y - 20) / 20;
        let l2 = (y - 20) % 20;
        assert!(l1 < 20);
        let l1 = ROWS[usize::from(l1)];
        let l2 = ROWS[usize::from(l2)];
        format!("{l1}{l2}{n}", n = x + 1)
    } else {
        let l = ROWS[usize::from(y)];
        format!("{l}{n}", n = x + 1)
    }
}

pub fn get_full_bga_pins(width: u8, height: u8) -> Vec<String> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| ball(x, y)))
        .collect()
}

pub fn get_seq_pins(num: u32) -> Vec<String> {
    (0..num).map(|x| (x + 1).to_string()).collect()
}

pub fn get_qn84_pins() -> Vec<String> {
    (0..48)
        .map(|x| format!("A{}", x + 1))
        .chain((0..36).map(|x| format!("B{}", x + 1)))
        .collect()
}

pub fn get_cb132_pins() -> Vec<String> {
    (0..14)
        .flat_map(|y| {
            (0..14)
                .filter(move |&x| match (x, y) {
                    (0 | 13, _) => true,
                    (_, 0 | 13) => true,
                    (1 | 12, _) => false,
                    (_, 1 | 12) => false,
                    (2 | 3 | 10 | 11, _) => true,
                    (_, 2 | 3 | 10 | 11) => true,
                    (4 | 9, _) => false,
                    (_, 4 | 9) => false,
                    _ => true,
                })
                .map(move |x| ball(x, y))
        })
        .collect()
}

pub fn get_cb284_pins() -> Vec<String> {
    (0..22)
        .flat_map(|y| {
            (0..22)
                .filter(move |&x| match (x, y) {
                    (0 | 21, _) => true,
                    (_, 0 | 21) => true,
                    (1 | 20, _) => false,
                    (_, 1 | 20) => false,
                    (2 | 19, _) => true,
                    (_, 2 | 19) => true,
                    (3 | 18, _) => false,
                    (_, 3 | 18) => false,
                    (4 | 17, _) => true,
                    (_, 4 | 17) => true,
                    (5 | 16, _) => false,
                    (_, 5 | 16) => false,
                    (6 | 7 | 14 | 15, _) => true,
                    (_, 6 | 7 | 14 | 15) => true,
                    (8 | 13, _) => false,
                    (_, 8 | 13) => false,
                    _ => true,
                })
                .map(move |x| ball(x, y))
        })
        .collect()
}

pub fn get_pkg_pins(pkg: &str) -> Vec<String> {
    match pkg {
        "SWG16" | "SWG16TR" => get_full_bga_pins(4, 4),
        "UWG20" => get_full_bga_pins(5, 4),
        "SWG25TR" => get_full_bga_pins(5, 5),
        "SWG30" => get_full_bga_pins(6, 5),
        "UWG30" => get_full_bga_pins(5, 6),
        "SWG36" => get_full_bga_pins(6, 6),
        "CM36" | "CM36A" | "CS36" | "CX36" | "CY36" | "FC36" => get_full_bga_pins(6, 6),
        "CM49" | "CM49A" | "FWG49" => get_full_bga_pins(7, 7),
        "CS63" => get_full_bga_pins(9, 7),
        "CC72" => get_full_bga_pins(9, 8),
        "CB81" | "CM81" => get_full_bga_pins(9, 9),
        "CS110" => get_full_bga_pins(11, 10),
        "BG121" | "CB121" | "CM121" => get_full_bga_pins(11, 11),
        "CB132" | "CB132R" => get_cb132_pins(),
        "CB196" => get_full_bga_pins(14, 14),
        "CM225" | "UMG225" => get_full_bga_pins(15, 15),
        "CT256" => get_full_bga_pins(16, 16),
        "CB284" => get_cb284_pins(),

        "QN32" => get_seq_pins(32),
        "SG48" => {
            let mut res = get_seq_pins(48);
            res.push("PAD".to_string());
            res
        }
        "QN84" | "QFN84" => get_qn84_pins(),

        "VQ100" => get_seq_pins(100),
        "TQ144" => get_seq_pins(144),

        _ => panic!("ummm {pkg}?"),
    }
}
