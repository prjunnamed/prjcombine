use std::fmt::Write;
use std::path::Path;

use prjcombine_types::bitvec::BitVec;

/// Represents the contents of a JESD3 file.
#[derive(Clone, Debug, Default)]
pub struct JedFile {
    /// The design specification: text after STX and before first field, not including
    /// the terminating `'*'`.
    ///
    /// A `None` signifies a non-standard variant of JESD3 where the design specification
    /// is completely missing, and the first field immediately follows STX.
    /// This variant is used by Xilinx.  Since there is no way to distinguish the two variants
    /// in the parser, it needs to be explicitly enabled in [`JedParserOptions`].
    pub design_spec: Option<String>,
    /// The list of notes encountered in the file.
    pub notes: Vec<String>,
    /// The fuses in the file.  A `None` signifies there's no `QF` field in the file.
    pub fuses: Option<BitVec>,
    /// If true, the `C` field is skipped even if there are fuses present.
    pub skip_fuse_checksum: bool,
    /// If true, the ETX checksum is skipped.  Note that "skipping" the checksum actually means
    /// setting it to `0000`.  Since `0000` also happens to be a valid checksum value, the parser
    /// cannot really distinguish the two cases.
    pub skip_etx_checksum: bool,
    /// The state of the security fuse: true (prevent readout), false (do not), or none
    /// (not specified in the file).
    pub security: Option<bool>,
    /// Electrical fuses.  If empty, not specified in the file.
    pub electrical: BitVec,
    /// User fuses.  If empty, not specified in the file.
    pub user: BitVec,
}

#[derive(Clone, Debug, Default)]
pub struct JedParserOptions {
    /// If true, parses a non-standard variant of JESD3 (used by Xilinx) where the design
    /// specification is missing.
    pub skip_design_spec: bool,
}

impl JedParserOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn skip_design_spec(self) -> Self {
        Self {
            skip_design_spec: true,
        }
    }
}

#[derive(Debug)]
pub enum JedParserError {
    StxMissing,
    EtxMissing,
    UnterminatedField,
    FuseLengthDuplicated,
    FuseDefaultSequenceError,
    FuseMissingLength,
    FuseMissingDefault,
    FuseOverrun,
    FuseChecksumMismatch,
    FuseSecurityDuplicated,
    FuseUserDuplicated,
    FuseElectricalDuplicated,
    EtxChecksumMissing,
    EtxChecksumMismatch,
    InvalidArgument,
    IoError(std::io::Error),
}

impl From<std::io::Error> for JedParserError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl std::fmt::Display for JedParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JedParserError::StxMissing => write!(f, "STX missing"),
            JedParserError::EtxMissing => write!(f, "ETX missing"),
            JedParserError::UnterminatedField => write!(f, "unterminated field"),
            JedParserError::FuseLengthDuplicated => write!(f, "fuse length duplicated"),
            JedParserError::FuseDefaultSequenceError => {
                write!(f, "fuse default specified after fuse data")
            }
            JedParserError::FuseMissingLength => write!(f, "fuse length missing"),
            JedParserError::FuseMissingDefault => write!(f, "fuse default missing"),
            JedParserError::FuseOverrun => write!(f, "fuse length overrun"),
            JedParserError::FuseChecksumMismatch => write!(f, "fuse checksum mismatch"),
            JedParserError::FuseSecurityDuplicated => write!(f, "security fuse duplicated"),
            JedParserError::FuseUserDuplicated => write!(f, "user fuse duplicated"),
            JedParserError::FuseElectricalDuplicated => write!(f, "electrical fuse duplicated"),
            JedParserError::EtxChecksumMissing => write!(f, "etx checksum missing"),
            JedParserError::EtxChecksumMismatch => write!(f, "etx checksum mismatch"),
            JedParserError::InvalidArgument => write!(f, "invalid argument"),
            JedParserError::IoError(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for JedParserError {}

impl JedFile {
    pub fn new() -> Self {
        JedFile::default()
    }

    pub fn with_fuses(self, fuses: BitVec) -> Self {
        Self {
            fuses: Some(fuses),
            ..self
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn fuse_checksum(&self) -> u16 {
        let mut checksum: u16 = 0;
        for (i, fuse) in self.fuses.as_ref().unwrap().iter().enumerate() {
            if fuse {
                checksum = checksum.wrapping_add(1 << (i % 8));
            }
        }
        checksum
    }

    pub fn emit(&self) -> String {
        let mut out = String::new();
        write!(out, "\x02").unwrap();
        if let Some(ref header) = self.design_spec {
            writeln!(out, "{header}*").unwrap();
        }
        for note in &self.notes {
            writeln!(out, "N{note}*").unwrap();
        }
        if let Some(ref fuses) = self.fuses {
            writeln!(out, "QF{n}*", n = fuses.len()).unwrap();
            writeln!(out, "F0*").unwrap();
            let mut pos = 0;
            while pos < fuses.len() {
                write!(out, "L{pos:06} ").unwrap();
                for _ in 0..80 {
                    if pos >= fuses.len() {
                        break;
                    }
                    write!(out, "{x}", x = u32::from(fuses[pos])).unwrap();
                    pos += 1;
                }
                writeln!(out, "*").unwrap();
            }
            if !self.skip_fuse_checksum {
                let checksum = self.fuse_checksum();
                writeln!(out, "C{checksum:04X}*").unwrap();
            }
        }
        if !self.electrical.is_empty() {
            writeln!(out, "E{}*", self.electrical).unwrap();
        }
        if !self.user.is_empty() {
            writeln!(out, "U{}*", self.user).unwrap();
        }
        if let Some(security) = self.security {
            writeln!(out, "G{security}*", security = u32::from(security)).unwrap();
        }
        write!(out, "\x03").unwrap();
        if self.skip_etx_checksum {
            writeln!(out, "0000").unwrap();
        } else {
            let mut checksum: u16 = 0;
            for &byte in out.as_bytes() {
                checksum = checksum.wrapping_add(byte.into());
            }
            writeln!(out, "{checksum:04X}").unwrap();
        }
        out
    }

    pub fn emit_to_file(&self, fname: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::write(fname, self.emit())
    }

    pub fn parse(jed: &str, options: &JedParserOptions) -> Result<JedFile, JedParserError> {
        let stx = jed.find('\x02').ok_or(JedParserError::StxMissing)?;
        let etx = jed[stx..].find('\x03').ok_or(JedParserError::EtxMissing)? + stx;
        let mut fuses: Option<BitVec> = None;
        let mut fuses_valid: Option<BitVec> = None;
        let mut notes = vec![];
        let mut position = stx + 1;
        let mut design_spec = None;
        let mut fuse_checksum = None;
        let mut electrical = BitVec::new();
        let mut user = BitVec::new();
        let mut security = None;
        if !options.skip_design_spec {
            let ds_end = position
                + jed[position..etx]
                    .find('*')
                    .ok_or(JedParserError::UnterminatedField)?;
            design_spec = Some(jed[position..ds_end].to_string());
            position = ds_end + 1;
        }
        loop {
            let Some(p) = jed[position..etx].find('*') else {
                let rest = jed[position..etx].trim();
                if !rest.is_empty() {
                    Err(JedParserError::UnterminatedField)?;
                }
                break;
            };
            let field_end = position + p;
            let field = &jed[position..field_end];
            position = field_end + 1;
            let field = field.trim_start();
            if let Some(arg) = field.strip_prefix("QF") {
                if fuses.is_some() {
                    Err(JedParserError::FuseLengthDuplicated)?;
                }
                let n: usize = arg.parse().unwrap();
                fuses = Some(BitVec::repeat(false, n));
                fuses_valid = Some(BitVec::repeat(false, n));
            } else if let Some(arg) = field.strip_prefix("N") {
                notes.push(arg.to_string());
            } else if let Some(arg) = field.strip_prefix('F') {
                let Some(ref cur_fuses_valid) = fuses_valid else {
                    Err(JedParserError::FuseMissingLength)?
                };
                if cur_fuses_valid.any() {
                    Err(JedParserError::FuseDefaultSequenceError)?
                }
                let val = match arg {
                    "0" => false,
                    "1" => true,
                    _ => Err(JedParserError::InvalidArgument)?,
                };
                fuses = Some(BitVec::repeat(val, cur_fuses_valid.len()));
                fuses_valid = Some(BitVec::repeat(true, cur_fuses_valid.len()));
            } else if let Some(arg) = field.strip_prefix('L') {
                let sp = arg.find(' ').ok_or(JedParserError::InvalidArgument)?;
                let mut pos: usize = arg[..sp]
                    .parse()
                    .map_err(|_| JedParserError::InvalidArgument)?;
                let fuses = fuses.as_mut().ok_or(JedParserError::FuseMissingLength)?;
                let fuses_valid = fuses_valid
                    .as_mut()
                    .ok_or(JedParserError::FuseMissingLength)?;
                for c in arg[sp..].chars() {
                    let val = match c {
                        '0' => false,
                        '1' => true,
                        ' ' | '\n' | '\r' => continue,
                        _ => Err(JedParserError::InvalidArgument)?,
                    };
                    if pos >= fuses.len() {
                        Err(JedParserError::FuseOverrun)?
                    }
                    fuses.set(pos, val);
                    fuses_valid.set(pos, true);
                    pos += 1;
                }
            } else if let Some(arg) = field.strip_prefix('C') {
                if arg.len() != 4 {
                    Err(JedParserError::InvalidArgument)?
                }
                let n =
                    u16::from_str_radix(arg, 16).map_err(|_| JedParserError::InvalidArgument)?;
                fuse_checksum = Some(n);
                if fuses.is_none() {
                    Err(JedParserError::FuseMissingLength)?
                }
            } else if let Some(arg) = field.strip_prefix('E') {
                if !electrical.is_empty() {
                    Err(JedParserError::FuseElectricalDuplicated)?
                }
                if let Some(arg) = arg.strip_prefix('H') {
                    for c in arg.chars().rev() {
                        let Some(c) = c.to_digit(16) else {
                            Err(JedParserError::InvalidArgument)?
                        };
                        for i in 0..4 {
                            electrical.push(((c >> i) & 1) != 0);
                        }
                    }
                } else {
                    for c in arg.chars().rev() {
                        let val = match c {
                            '0' => false,
                            '1' => true,
                            _ => Err(JedParserError::InvalidArgument)?,
                        };
                        electrical.push(val);
                    }
                }
            } else if let Some(arg) = field.strip_prefix('U') {
                if !user.is_empty() {
                    Err(JedParserError::FuseUserDuplicated)?
                }
                if let Some(arg) = arg.strip_prefix('H') {
                    for c in arg.chars().rev() {
                        let Some(c) = c.to_digit(16) else {
                            Err(JedParserError::InvalidArgument)?
                        };
                        for i in 0..4 {
                            user.push(((c >> i) & 1) != 0);
                        }
                    }
                } else if let Some(arg) = arg.strip_prefix('A') {
                    for c in arg.chars().rev() {
                        let c: u32 = c.into();
                        if c >= 0x80 {
                            Err(JedParserError::InvalidArgument)?
                        }
                        for i in 0..7 {
                            user.push(((c >> i) & 1) != 0);
                        }
                    }
                } else {
                    for c in arg.chars().rev() {
                        let val = match c {
                            '0' => false,
                            '1' => true,
                            _ => Err(JedParserError::InvalidArgument)?,
                        };
                        user.push(val);
                    }
                }
            } else if let Some(arg) = field.strip_prefix('G') {
                if security.is_some() {
                    Err(JedParserError::FuseSecurityDuplicated)?
                }
                security = Some(match arg {
                    "0" => false,
                    "1" => true,
                    _ => Err(JedParserError::InvalidArgument)?,
                });
            }
        }
        if let Some(fuses_valid) = fuses_valid {
            if !fuses_valid.all() {
                Err(JedParserError::FuseMissingDefault)?
            }
        }
        if etx + 5 > jed.len() {
            Err(JedParserError::EtxChecksumMissing)?
        }
        let etx_checksum = u16::from_str_radix(&jed[etx + 1..etx + 5], 16)
            .map_err(|_| JedParserError::InvalidArgument)?;
        let mut checksum: u16 = 0;
        for &byte in &jed.as_bytes()[stx..etx + 1] {
            checksum = checksum.wrapping_add(byte.into());
        }
        let mut skip_etx_checksum = false;
        if checksum != etx_checksum {
            if etx_checksum == 0 {
                skip_etx_checksum = true;
            } else {
                Err(JedParserError::EtxChecksumMismatch)?
            }
        }
        let res = JedFile {
            design_spec,
            notes,
            fuses,
            skip_fuse_checksum: fuse_checksum.is_none(),
            skip_etx_checksum,
            electrical,
            user,
            security,
        };
        if let Some(checksum) = fuse_checksum {
            if checksum != res.fuse_checksum() {
                Err(JedParserError::FuseChecksumMismatch)?
            }
        }
        Ok(res)
    }

    pub fn parse_from_file(
        fname: impl AsRef<Path>,
        options: &JedParserOptions,
    ) -> Result<Self, JedParserError> {
        let jed = std::fs::read_to_string(fname)?;
        Self::parse(&jed, options)
    }
}
