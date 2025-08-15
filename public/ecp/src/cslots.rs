use prjcombine_interconnect::connector_slots;

connector_slots![
    W: E,
    E: W,
    S: N,
    N: S,
    SW: SE,
    SE: SW,
    EBR_W: EBR_E,
    EBR_E: EBR_W,
    IO_W: IO_E,
    IO_E: IO_W,
];
