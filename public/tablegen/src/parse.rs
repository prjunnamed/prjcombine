use prjcombine_interconnect::db::PadKind;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{BitRectGeometry, FrameOrientation},
};
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::ast::{self, IfCond};

type Result<T> = core::result::Result<T, TokenStream>;

pub trait Item: From<ast::If<Self>> + From<ast::ForLoop<Self>> {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self>;
}

pub fn parse_str(lit: &Literal) -> Result<String> {
    let s = lit.to_string();
    let Some(s) = s.strip_prefix('"') else {
        error_at(lit.span(), "expected string")?
    };
    let Some(s) = s.strip_suffix('"') else {
        error_at(lit.span(), "expected string")?
    };
    if s.contains('\\') {
        error_at(lit.span(), "escape sequences not supported")?
    }
    Ok(s.to_string())
}

struct Tokenizer {
    span: Span,
    tokens: core::iter::Peekable<std::vec::IntoIter<TokenTree>>,
}

impl Tokenizer {
    fn new(span: Span, tokens: Vec<TokenTree>) -> Self {
        Self {
            span,
            tokens: tokens.into_iter().peekable(),
        }
    }

    fn next(&mut self) -> Option<TokenTree> {
        if let Some(token) = self.tokens.next() {
            self.span = token.span();
            Some(token.clone())
        } else {
            None
        }
    }

    fn is_empty(&mut self) -> bool {
        self.tokens.peek().is_none()
    }

    fn error<T>(&mut self, msg: &str) -> Result<T> {
        error_at(self.span, msg)
    }

    fn try_ident(&mut self) -> Option<Ident> {
        if let Some(TokenTree::Ident(id)) = self.tokens.peek() {
            let id = id.clone();
            self.next();
            Some(id)
        } else {
            None
        }
    }

    fn ident(&mut self) -> Result<Ident> {
        if let Some(ident) = self.try_ident() {
            Ok(ident)
        } else {
            self.error("expected identifier")?
        }
    }

    fn try_template_id(&mut self) -> Option<ast::TemplateId> {
        if let Some(ident) = self.try_ident() {
            Some(ast::TemplateId::Raw(ident))
        } else if let Some(TokenTree::Literal(lit)) = self.tokens.peek()
            && lit.to_string().starts_with('"')
        {
            let lit = lit.clone();
            self.next();
            Some(ast::TemplateId::String(lit))
        } else {
            None
        }
    }

    fn template_id(&mut self) -> Result<ast::TemplateId> {
        if let Some(ident) = self.try_template_id() {
            Ok(ident)
        } else {
            self.error("expected identifier")?
        }
    }

    fn list<T>(
        &mut self,
        mut parse_item: impl FnMut(&mut Tokenizer) -> Result<T>,
    ) -> Result<Vec<T>> {
        let mut res = vec![];
        res.push(parse_item(self)?);
        while self.try_punct(',') {
            res.push(parse_item(self)?);
        }
        Ok(res)
    }

    fn template_id_list(&mut self) -> Result<Vec<ast::TemplateId>> {
        self.list(Tokenizer::template_id)
    }

    fn array_id_def(&mut self) -> Result<ast::ArrayIdDef> {
        let id = self.template_id()?;
        Ok(
            if let Some(TokenTree::Group(g)) = self.tokens.peek()
                && g.delimiter() == Delimiter::Bracket
            {
                let mut tokenizer = Tokenizer::new(g.span(), Vec::from_iter(g.stream()));
                let num = tokenizer.usize()?;
                tokenizer.finish()?;
                self.next();
                ast::ArrayIdDef::Array(id, num)
            } else {
                ast::ArrayIdDef::Plain(id)
            },
        )
    }

    fn array_id_ref(&mut self) -> Result<ast::ArrayIdRef> {
        let id = self.template_id()?;
        Ok(if let Some(index) = self.try_index()? {
            ast::ArrayIdRef::Indexed(id, index)
        } else {
            ast::ArrayIdRef::Plain(id)
        })
    }

    fn try_index(&mut self) -> Result<Option<ast::Index>> {
        let Some(mut inner) = self.try_brackets() else {
            return Ok(None);
        };
        Ok(Some(if let Some(id) = inner.try_ident() {
            if inner.try_punct('+') {
                let offset = inner.usize()?;
                inner.finish()?;
                ast::Index::Ident(id, offset)
            } else {
                inner.finish()?;
                ast::Index::Ident(id, 0)
            }
        } else {
            let n = inner.usize()?;
            inner.finish()?;
            ast::Index::Literal(n)
        }))
    }

    fn wire_ref(&mut self) -> Result<ast::WireRef> {
        let ref1 = self.array_id_ref()?;
        Ok(if self.try_punct('.') {
            let ref2 = self.array_id_ref()?;
            ast::WireRef::Qualified(ref1, ref2)
        } else {
            ast::WireRef::Simple(ref1)
        })
    }

    fn pol_wire_ref(&mut self) -> Result<ast::PolWireRef> {
        let inv = self.try_punct('~');
        let wire = self.wire_ref()?;
        Ok(if inv {
            ast::PolWireRef::Neg(wire)
        } else {
            ast::PolWireRef::Pos(wire)
        })
    }

    fn try_kw(&mut self, kw: &str) -> bool {
        if let Some(TokenTree::Ident(id)) = self.tokens.peek()
            && id.to_string() == kw
        {
            self.next();
            true
        } else {
            false
        }
    }

    fn kw(&mut self, kw: &str) -> Result<()> {
        if !self.try_kw(kw) {
            self.error(&format!("expected {kw}"))?
        }
        Ok(())
    }

    fn usize(&mut self) -> Result<usize> {
        let Some(TokenTree::Literal(lit)) = self.next() else {
            self.error("expected number")?
        };
        lit.to_string()
            .parse()
            .map_err(|_| self.error::<()>("expected number").unwrap_err())
    }

    fn bitvec(&mut self) -> Result<BitVec> {
        let Some(TokenTree::Literal(lit)) = self.next() else {
            self.error("expected bitvec")?
        };
        let s = lit.to_string();
        Ok(if let Some(s) = s.strip_prefix("0b") {
            let mut res = BitVec::new();
            for c in s.chars().rev() {
                match c {
                    '0' => res.push(false),
                    '1' => res.push(true),
                    '_' => (),
                    _ => self.error("expected bitvec")?,
                }
            }
            res
        } else if s == "0" {
            bits![0]
        } else if s == "1" {
            bits![1]
        } else {
            self.error("expected bitvec")?
        })
    }

    fn try_punct(&mut self, ch: char) -> bool {
        if let Some(TokenTree::Punct(p)) = self.tokens.peek()
            && p.as_char() == ch
        {
            self.next();
            true
        } else {
            false
        }
    }

    fn punct(&mut self, ch: char) -> Result<()> {
        if !self.try_punct(ch) {
            self.error(&format!("expected '{ch}'"))?
        }
        Ok(())
    }

    fn parens(&mut self) -> Result<Tokenizer> {
        let Some(TokenTree::Group(g)) = self.next() else {
            self.error("expected parenthesized group")?
        };
        if g.delimiter() != Delimiter::Parenthesis {
            self.error("expected parenthesized group")?
        }
        Ok(Tokenizer::new(g.span(), Vec::from_iter(g.stream())))
    }

    fn try_brackets(&mut self) -> Option<Tokenizer> {
        let Some(TokenTree::Group(g)) = self.tokens.peek() else {
            return None;
        };
        if g.delimiter() != Delimiter::Bracket {
            return None;
        }
        let res = Tokenizer::new(g.span(), Vec::from_iter(g.stream()));
        self.next();
        Some(res)
    }

    fn brackets(&mut self) -> Result<Tokenizer> {
        let Some(TokenTree::Group(g)) = self.next() else {
            self.error("expected bracketed group")?
        };
        if g.delimiter() != Delimiter::Bracket {
            self.error("expected bracketed group")?
        }
        Ok(Tokenizer::new(g.span(), Vec::from_iter(g.stream())))
    }

    fn finish(&mut self) -> Result<()> {
        if let Some(token) = self.next() {
            self.error(&format!("unexpected '{token}'"))?;
        }
        Ok(())
    }
}

// region: top items

fn parse_bitrect_class(mut tokenizer: Tokenizer) -> Result<ast::BitRectClass> {
    let name = tokenizer.ident()?;
    tokenizer.punct('=')?;
    let orientation = tokenizer.ident()?;
    let orientation = match orientation.to_string().as_str() {
        "horizontal" => FrameOrientation::Horizontal,
        "vertical" => FrameOrientation::Vertical,
        _ => error_at(
            orientation.span(),
            &format!("unknown orientation {orientation}"),
        )?,
    };
    let mut dims = tokenizer.parens()?;
    tokenizer.finish()?;

    let rev_frames = dims.try_kw("rev");
    let frames = dims.usize()?;
    dims.punct(',')?;
    let rev_bits = dims.try_kw("rev");
    let bits = dims.usize()?;

    dims.finish()?;

    Ok(ast::BitRectClass {
        name,
        geometry: BitRectGeometry {
            frames,
            bits,
            orientation,
            rev_frames,
            rev_bits,
        },
    })
}

fn parse_enum(mut tokenizer: Tokenizer, block: TokenStream) -> Result<ast::EnumClass> {
    let name = tokenizer.ident()?;
    tokenizer.finish()?;
    let mut values = vec![];

    tokenizer = Tokenizer::new(name.span(), Vec::from_iter(block));
    while !tokenizer.is_empty() {
        values.push(tokenizer.ident()?);
        if !tokenizer.is_empty() {
            tokenizer.punct(',')?;
        }
    }
    tokenizer.finish()?;

    Ok(ast::EnumClass { name, values })
}

fn parse_region_slot(mut tokenizer: Tokenizer) -> Result<ast::RegionSlot> {
    let name = tokenizer.template_id()?;
    tokenizer.finish()?;
    Ok(ast::RegionSlot { name })
}

impl Item for ast::TopItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "variant" => {
                if block.is_some() {
                    error_at(keyword.span(), "variant does not accept a block")?;
                }
                let id = tokenizer.ident()?;
                tokenizer.finish()?;
                ast::TopItem::Variant(id)
            }
            "bitrect" => {
                if block.is_some() {
                    error_at(keyword.span(), "bitrect does not accept a block")?;
                }
                ast::TopItem::BitRectClass(parse_bitrect_class(tokenizer)?)
            }
            "enum" => {
                let Some(block) = block else {
                    error_at(keyword.span(), "enum requires a block")?
                };
                ast::TopItem::EnumClass(parse_enum(tokenizer, block)?)
            }
            "bel_class" => ast::TopItem::BelClass(parse_bel_class(tokenizer, block)?),
            "tile_slot" => ast::TopItem::TileSlot(parse_tile_slot(tokenizer, block)?),
            "connector_slot" => {
                ast::TopItem::ConnectorSlot(parse_connector_slot(tokenizer, block)?)
            }
            "region_slot" => {
                if block.is_some() {
                    error_at(keyword.span(), "region_slot does not accept a block")?;
                }
                ast::TopItem::RegionSlot(parse_region_slot(tokenizer)?)
            }
            "wire" => {
                if block.is_some() {
                    error_at(keyword.span(), "wire does not accept a block")?;
                }
                ast::TopItem::Wire(parse_wire(tokenizer)?)
            }
            "table" => {
                let Some(block) = block else {
                    error_at(keyword.span(), "table requires a block")?
                };
                ast::TopItem::Table(parse_table(tokenizer, block)?)
            }
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

// endregion

// region: bel class

fn parse_bel_class(mut tokenizer: Tokenizer, block: Option<TokenStream>) -> Result<ast::BelClass> {
    let name = tokenizer.ident()?;
    tokenizer.finish()?;

    let items = if let Some(block) = block {
        parse(block)?
    } else {
        vec![]
    };
    Ok(ast::BelClass { name, items })
}

impl Item for ast::BelClassItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "nonroutable" | "input" | "output" | "bidir" => {
                if block.is_some() {
                    error_at(keyword.span(), "pin does not accept a block")?;
                }
                let mut keyword = keyword;
                let nonroutable = if keyword.to_string() == "nonroutable" {
                    keyword = tokenizer.ident()?;
                    true
                } else {
                    false
                };
                let names = tokenizer.list(Tokenizer::array_id_def)?;
                tokenizer.finish()?;
                let pin = ast::BelClassPin { names, nonroutable };
                match keyword.to_string().as_str() {
                    "input" => ast::BelClassItem::Input(pin),
                    "output" => ast::BelClassItem::Output(pin),
                    "bidir" => ast::BelClassItem::Bidir(pin),
                    _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
                }
            }
            "pad" => {
                if block.is_some() {
                    error_at(keyword.span(), "pad does not accept a block")?;
                }
                let names = tokenizer.list(Tokenizer::array_id_def)?;
                tokenizer.punct(':')?;
                let kind_raw = tokenizer.ident()?;
                let kind = match kind_raw.to_string().as_str() {
                    "input" => PadKind::In,
                    "output" => PadKind::Out,
                    "inout" => PadKind::Inout,
                    "power" => PadKind::Power,
                    "analog" => PadKind::Analog,
                    _ => error_at(kind_raw.span(), "unknown pad kind")?,
                };
                tokenizer.finish()?;
                ast::BelClassItem::Pad(ast::BelClassPad { names, kind })
            }
            "attribute" => {
                if block.is_some() {
                    error_at(keyword.span(), "attribute does not accept a block")?;
                }
                let names = tokenizer.list(Tokenizer::template_id)?;
                tokenizer.punct(':')?;
                let typ_raw = tokenizer.ident()?;
                let typ = match typ_raw.to_string().as_str() {
                    "bool" => ast::AttributeType::Bool,
                    "bitvec" => {
                        let mut inner = tokenizer.brackets()?;
                        let width = inner.usize()?;
                        inner.finish()?;
                        ast::AttributeType::BitVec(width)
                    }
                    _ => ast::AttributeType::Enum(typ_raw),
                };
                tokenizer.finish()?;
                ast::BelClassItem::Attribute(ast::BelClassAttribute { names, typ })
            }
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

// endregion

// region: tiles

fn parse_switchbox(mut tokenizer: Tokenizer, block: Option<TokenStream>) -> Result<ast::SwitchBox> {
    let slot = tokenizer.array_id_ref()?;
    tokenizer.finish()?;

    let items = if let Some(block) = block {
        parse(block)?
    } else {
        vec![]
    };
    Ok(ast::SwitchBox { slot, items })
}

impl Item for ast::SwitchBoxItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "progbuf" => {
                if block.is_some() {
                    error_at(keyword.span(), "progbuf does not accept a block")?;
                }
                let wire_to = tokenizer.wire_ref()?;
                tokenizer.punct('=')?;
                let wire_from = tokenizer.pol_wire_ref()?;
                tokenizer.finish()?;
                ast::SwitchBoxItem::ProgBuf(wire_to, wire_from)
            }
            "permabuf" => {
                if block.is_some() {
                    error_at(keyword.span(), "permabuf does not accept a block")?;
                }
                let wire_to = tokenizer.wire_ref()?;
                tokenizer.punct('=')?;
                let wire_from = tokenizer.pol_wire_ref()?;
                tokenizer.finish()?;
                ast::SwitchBoxItem::PermaBuf(wire_to, wire_from)
            }
            "proginv" => {
                if block.is_some() {
                    error_at(keyword.span(), "proginv does not accept a block")?;
                }
                let wire_to = tokenizer.wire_ref()?;
                tokenizer.punct('=')?;
                let wire_from = tokenizer.wire_ref()?;
                tokenizer.finish()?;
                ast::SwitchBoxItem::ProgInv(wire_to, wire_from)
            }
            "mux" => {
                if block.is_some() {
                    error_at(keyword.span(), "mux does not accept a block")?;
                }
                let wire_to = tokenizer.wire_ref()?;
                tokenizer.punct('=')?;
                let mut wires_from = vec![tokenizer.pol_wire_ref()?];
                while tokenizer.try_punct('|') {
                    wires_from.push(tokenizer.pol_wire_ref()?);
                }
                tokenizer.finish()?;
                ast::SwitchBoxItem::Mux(wire_to, wires_from)
            }
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

fn parse_bel(mut tokenizer: Tokenizer, block: Option<TokenStream>) -> Result<ast::Bel> {
    let slot = tokenizer.array_id_ref()?;
    tokenizer.finish()?;

    let items = if let Some(block) = block {
        parse(block)?
    } else {
        vec![]
    };
    Ok(ast::Bel { slot, items })
}

fn parse_bel_attribute(
    mut tokenizer: Tokenizer,
    block: Option<TokenStream>,
) -> Result<ast::BelAttribute> {
    let name = tokenizer.template_id()?;
    tokenizer.punct('@')?;
    let mut bits = vec![];
    if let Some(mut inner) = tokenizer.try_brackets() {
        bits.push(parse_tilebit(&mut inner)?);
        while inner.try_punct(',') {
            if inner.is_empty() {
                break;
            }
            bits.push(parse_tilebit(&mut inner)?);
        }
        bits.reverse();
        inner.finish()?;
    } else {
        bits.push(parse_tilebit(&mut tokenizer)?);
    };
    tokenizer.finish()?;
    let values = if let Some(block) = block {
        let mut inner = Tokenizer::new(name.span(), Vec::from_iter(block));
        let mut values = vec![];
        while !inner.is_empty() {
            let vname = inner.template_id()?;
            inner.punct('=')?;
            let val = inner.bitvec()?;
            inner.punct(',')?;
            values.push((vname, val));
        }
        inner.finish()?;
        Some(values)
    } else {
        None
    };
    Ok(ast::BelAttribute { name, bits, values })
}

fn parse_tilebit(tokenizer: &mut Tokenizer) -> Result<ast::TileBit> {
    let inv = tokenizer.try_punct('!');
    let name = tokenizer.template_id()?;
    let mut index = vec![];
    while let Some(idx) = tokenizer.try_index()? {
        index.push(idx);
    }
    Ok(ast::TileBit { name, index, inv })
}

impl Item for ast::BelItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "input" => {
                if block.is_some() {
                    error_at(keyword.span(), "pin does not accept a block")?;
                }
                let pin = tokenizer.array_id_ref()?;
                tokenizer.punct('=')?;
                let wire = tokenizer.pol_wire_ref()?;
                tokenizer.finish()?;
                ast::BelItem::Input(pin, wire)
            }
            "output" => {
                if block.is_some() {
                    error_at(keyword.span(), "pin does not accept a block")?;
                }
                let pin = tokenizer.array_id_ref()?;
                tokenizer.punct('=')?;
                let wires = tokenizer.list(Tokenizer::wire_ref)?;
                tokenizer.finish()?;
                ast::BelItem::Output(pin, wires)
            }
            "bidir" => {
                if block.is_some() {
                    error_at(keyword.span(), "pin does not accept a block")?;
                }
                let pin = tokenizer.array_id_ref()?;
                tokenizer.punct('=')?;
                let wire = tokenizer.wire_ref()?;
                tokenizer.finish()?;
                ast::BelItem::Bidir(pin, wire)
            }
            "attribute" => ast::BelItem::Attribute(parse_bel_attribute(tokenizer, block)?),
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

fn parse_tile_class(
    mut tokenizer: Tokenizer,
    block: Option<TokenStream>,
) -> Result<ast::TileClass> {
    let names = tokenizer.template_id_list()?;
    tokenizer.finish()?;

    let items = if let Some(block) = block {
        parse(block)?
    } else {
        vec![]
    };
    Ok(ast::TileClass { names, items })
}

impl Item for ast::TileClassItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "cell" => {
                if block.is_some() {
                    error_at(keyword.span(), "cell does not accept a block")?;
                }
                let names = tokenizer.list(Tokenizer::array_id_def)?;
                tokenizer.finish()?;
                ast::TileClassItem::Cell(names)
            }
            "bitrect" => {
                if block.is_some() {
                    error_at(keyword.span(), "bitrect does not accept a block")?;
                }
                let name = tokenizer.array_id_def()?;
                tokenizer.punct(':')?;
                let class = tokenizer.ident()?;
                tokenizer.finish()?;
                ast::TileClassItem::BitRect(name, class)
            }
            "switchbox" => ast::TileClassItem::SwitchBox(parse_switchbox(tokenizer, block)?),
            "bel" => ast::TileClassItem::Bel(parse_bel(tokenizer, block)?),
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

fn parse_bel_slot(mut tokenizer: Tokenizer) -> Result<ast::BelSlot> {
    let name = tokenizer.array_id_def()?;
    tokenizer.punct(':')?;
    let kind = if tokenizer.try_kw("routing") {
        ast::BelKind::Routing
    } else if tokenizer.try_kw("legacy") {
        ast::BelKind::Legacy
    } else {
        let bcls = tokenizer.ident()?;
        ast::BelKind::Class(bcls)
    };
    tokenizer.finish()?;

    Ok(ast::BelSlot { name, kind })
}

fn parse_tile_slot(mut tokenizer: Tokenizer, block: Option<TokenStream>) -> Result<ast::TileSlot> {
    let name = tokenizer.template_id()?;
    tokenizer.finish()?;

    let items = if let Some(block) = block {
        parse(block)?
    } else {
        vec![]
    };
    Ok(ast::TileSlot { name, items })
}

impl Item for ast::TileSlotItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "bel_slot" => {
                if block.is_some() {
                    error_at(keyword.span(), "bel slot does not accept a block")?;
                }
                ast::TileSlotItem::BelSlot(parse_bel_slot(tokenizer)?)
            }
            "tile_class" => ast::TileSlotItem::TileClass(parse_tile_class(tokenizer, block)?),
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

// endregion

// region: connectors and wires

fn parse_connector_slot(
    mut tokenizer: Tokenizer,
    block: Option<TokenStream>,
) -> Result<ast::ConnectorSlot> {
    let name = tokenizer.template_id()?;
    tokenizer.finish()?;

    let items = if let Some(block) = block {
        parse(block)?
    } else {
        vec![]
    };
    Ok(ast::ConnectorSlot { name, items })
}

impl Item for ast::ConnectorSlotItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "opposite" => {
                if block.is_some() {
                    error_at(keyword.span(), "opposite does not accept a block")?;
                }
                let ident = tokenizer.template_id()?;
                tokenizer.finish()?;
                ast::ConnectorSlotItem::Opposite(ident)
            }
            "connector_class" => {
                ast::ConnectorSlotItem::ConnectorClass(parse_connector_class(tokenizer, block)?)
            }
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

fn parse_connector_class(
    mut tokenizer: Tokenizer,
    block: Option<TokenStream>,
) -> Result<ast::ConnectorClass> {
    let names = tokenizer.template_id_list()?;
    tokenizer.finish()?;

    let items = if let Some(block) = block {
        parse(block)?
    } else {
        vec![]
    };
    Ok(ast::ConnectorClass { names, items })
}

impl Item for ast::ConnectorClassItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "pass" => {
                if block.is_some() {
                    error_at(keyword.span(), "pass does not accept a block")?;
                }
                let dst = tokenizer.array_id_ref()?;
                tokenizer.punct('=')?;
                let src = tokenizer.array_id_ref()?;
                tokenizer.finish()?;
                ast::ConnectorClassItem::Pass(dst, src)
            }
            "reflect" => {
                if block.is_some() {
                    error_at(keyword.span(), "reflect does not accept a block")?;
                }
                let dst = tokenizer.array_id_ref()?;
                tokenizer.punct('=')?;
                let src = tokenizer.array_id_ref()?;
                tokenizer.finish()?;
                ast::ConnectorClassItem::Reflect(dst, src)
            }
            "blackhole" => {
                if block.is_some() {
                    error_at(keyword.span(), "blackhole does not accept a block")?;
                }
                let dst = tokenizer.array_id_ref()?;
                tokenizer.finish()?;
                ast::ConnectorClassItem::Blackhole(dst)
            }
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

fn parse_wire(mut tokenizer: Tokenizer) -> Result<ast::Wire> {
    let name = tokenizer.array_id_def()?;
    tokenizer.punct(':')?;
    let kind = tokenizer.ident()?;
    let kind = match kind.to_string().as_str() {
        "tie" => {
            let val = tokenizer.usize()?;
            match val {
                0 => ast::WireKind::Tie0,
                1 => ast::WireKind::Tie1,
                _ => tokenizer.error(&format!("invalid tie value {val}"))?,
            }
        }
        "regional" => ast::WireKind::Regional(tokenizer.template_id()?),
        "pullup" => ast::WireKind::TiePullup,
        "mux" => ast::WireKind::Mux,
        "bel" => ast::WireKind::Bel,
        "test" => ast::WireKind::Test,
        "multi_root" => ast::WireKind::MultiRoot,
        "branch" => ast::WireKind::Branch(tokenizer.ident()?),
        "multi_branch" => ast::WireKind::MultiBranch(tokenizer.ident()?),
        _ => error_at(kind.span(), &format!("unknown wire kind {kind}"))?,
    };
    tokenizer.finish()?;

    Ok(ast::Wire { name, kind })
}

// endregion

// region: tables

fn parse_table(mut tokenizer: Tokenizer, block: TokenStream) -> Result<ast::Table> {
    let name = tokenizer.template_id()?;
    tokenizer.finish()?;

    let items = parse(block)?;
    Ok(ast::Table { name, items })
}

impl Item for ast::TableItem {
    fn parse_item(
        keyword: Ident,
        tokens: Vec<TokenTree>,
        block: Option<TokenStream>,
    ) -> Result<Self> {
        let mut tokenizer = Tokenizer::new(keyword.span(), tokens);
        Ok(match keyword.to_string().as_str() {
            "field" => {
                if block.is_some() {
                    error_at(keyword.span(), "field does not accept a block")?;
                }
                let names = tokenizer.list(Tokenizer::template_id)?;
                tokenizer.punct(':')?;
                let typ_raw = tokenizer.ident()?;
                let typ = match typ_raw.to_string().as_str() {
                    "bool" => ast::AttributeType::Bool,
                    "bitvec" => {
                        let mut inner = tokenizer.brackets()?;
                        let width = inner.usize()?;
                        inner.finish()?;
                        ast::AttributeType::BitVec(width)
                    }
                    _ => ast::AttributeType::Enum(typ_raw),
                };
                tokenizer.finish()?;
                ast::TableItem::Field(ast::TableField {
                    names,
                    typ,
                })
            }
            "row" => {
                if block.is_some() {
                    todo!("row block")
                }
                let names = tokenizer.list(Tokenizer::template_id)?;
                tokenizer.finish()?;
                ast::TableItem::Row(names)
            }
            _ => error_at(keyword.span(), &format!("unknown item keyword: {keyword}"))?,
        })
    }
}

// endregion:

pub fn error_at<T>(span: Span, msg: &str) -> Result<T> {
    let mut group = Group::new(
        Delimiter::Parenthesis,
        TokenStream::from_iter([TokenTree::Literal(Literal::string(msg))]),
    );
    group.set_span(span);
    Err(TokenStream::from_iter([
        TokenTree::Ident(Ident::new("compile_error", span)),
        TokenTree::Punct(Punct::new('!', Spacing::Alone)),
        TokenTree::Group(group),
        TokenTree::Punct(Punct::new(';', Spacing::Alone)),
    ]))
}

struct IfBranch {
    span: Span,
    cond: Vec<TokenTree>,
    block: TokenStream,
}

fn parse_if<T: Item>(pre_branches: Vec<IfBranch>, else_block: Option<TokenStream>) -> Result<T> {
    let mut branches = vec![];
    let span = pre_branches[0].span;
    for branch in pre_branches {
        let mut tokenizer = Tokenizer::new(branch.span, branch.cond);
        let cond = tokenizer.ident()?;
        let cond = match cond.to_string().as_str() {
            "variant" => {
                let mut ids = vec![];
                if let Some(mut inner) = tokenizer.try_brackets() {
                    ids = inner.list(Tokenizer::ident)?;
                    inner.finish()?;
                } else {
                    ids.push(tokenizer.ident()?);
                }
                IfCond::Variant(ids)
            }
            "tile_class" => {
                let mut ids = vec![];
                if let Some(id) = tokenizer.try_template_id() {
                    ids.push(id);
                } else {
                    let mut inner = tokenizer.brackets()?;
                    ids = inner.list(Tokenizer::template_id)?;
                    inner.finish()?;
                }
                IfCond::TileClass(ids)
            }
            "bel_slot" => {
                let mut ids = vec![];
                if let Some(mut inner) = tokenizer.try_brackets() {
                    ids = inner.list(Tokenizer::array_id_ref)?;
                    inner.finish()?;
                } else {
                    ids.push(tokenizer.array_id_ref()?);
                }
                IfCond::BelSlot(ids)
            }
            _ => error_at(cond.span(), &format!("unknown condition {cond}"))?,
        };
        tokenizer.finish()?;
        branches.push((cond, parse(branch.block)?));
    }
    let else_items = if let Some(block) = else_block {
        parse(block)?
    } else {
        vec![]
    };
    Ok(ast::If {
        span,
        branches,
        else_items,
    }
    .into())
}

fn parse_for<T: Item>(span: Span, tokens: Vec<TokenTree>, block: TokenStream) -> Result<T> {
    let mut tokenizer = Tokenizer::new(span, tokens);
    let var = tokenizer.ident()?;
    tokenizer.kw("in")?;

    let n1 = tokenizer.usize()?;
    tokenizer.punct('.')?;
    tokenizer.punct('.')?;
    let is_inclusive = tokenizer.try_punct('=');
    let n2 = tokenizer.usize()?;

    let iterator = if is_inclusive {
        ast::ForIterator::RangeInclusive(n1..=n2)
    } else {
        ast::ForIterator::Range(n1..n2)
    };

    tokenizer.finish()?;
    Ok(ast::ForLoop {
        var,
        iterator,
        items: parse(block)?,
    }
    .into())
}

pub fn parse<T: Item>(tokens: TokenStream) -> Result<Vec<T>> {
    let mut res = vec![];
    enum State {
        Initial,
        Item(Ident, Vec<TokenTree>),
        ForLoop(Ident, Vec<TokenTree>),
        If(Ident, Vec<IfBranch>, Vec<TokenTree>),
        IfMaybeDone(Vec<IfBranch>),
        Else(Ident, Vec<IfBranch>),
    }
    let mut state = State::Initial;
    for token in tokens {
        'retry: loop {
            state = match state {
                State::Initial => {
                    if let TokenTree::Ident(kw) = token {
                        if kw.to_string() == "if" {
                            State::If(kw, vec![], vec![])
                        } else if kw.to_string() == "for" {
                            State::ForLoop(kw, vec![])
                        } else {
                            State::Item(kw, vec![])
                        }
                    } else {
                        error_at(token.span(), "expected keyword")?
                    }
                }
                State::Item(kw, mut item_tokens) => match token {
                    TokenTree::Punct(ref p) if p.as_char() == ';' => {
                        res.push(T::parse_item(kw, item_tokens, None)?);
                        State::Initial
                    }
                    TokenTree::Group(ref g) if g.delimiter() == Delimiter::Brace => {
                        res.push(T::parse_item(kw, item_tokens, Some(g.stream()))?);
                        State::Initial
                    }
                    _ => {
                        item_tokens.push(token);
                        State::Item(kw, item_tokens)
                    }
                },
                State::ForLoop(kw, mut item_tokens) => match token {
                    TokenTree::Group(ref g) if g.delimiter() == Delimiter::Brace => {
                        res.push(parse_for(kw.span(), item_tokens, g.stream())?);
                        State::Initial
                    }
                    _ => {
                        item_tokens.push(token);
                        State::ForLoop(kw, item_tokens)
                    }
                },
                State::If(kw, mut branches, mut cond) => {
                    if let TokenTree::Group(ref g) = token
                        && g.delimiter() == Delimiter::Brace
                    {
                        branches.push(IfBranch {
                            span: kw.span(),
                            cond,
                            block: g.stream(),
                        });
                        State::IfMaybeDone(branches)
                    } else {
                        cond.push(token);
                        State::If(kw, branches, cond)
                    }
                }
                State::IfMaybeDone(branches) => {
                    if let TokenTree::Ident(ref kw) = token
                        && kw.to_string() == "else"
                    {
                        State::Else(kw.clone(), branches)
                    } else {
                        res.push(parse_if(branches, None)?);
                        state = State::Initial;
                        continue 'retry;
                    }
                }
                State::Else(_kw, branches) => {
                    if let TokenTree::Ident(ref kw) = token
                        && kw.to_string() == "if"
                    {
                        State::If(kw.clone(), branches, vec![])
                    } else if let TokenTree::Group(ref group) = token
                        && group.delimiter() == Delimiter::Brace
                    {
                        res.push(parse_if(branches, Some(group.stream()))?);
                        State::Initial
                    } else {
                        error_at(token.span(), "expected `if` or block")?
                    }
                }
            };
            break;
        }
    }
    match state {
        State::Initial => (),
        State::Item(kw, _) => error_at(kw.span(), "unfinished item")?,
        State::ForLoop(kw, _) => error_at(kw.span(), "unfinished for")?,
        State::If(kw, _, _) => error_at(kw.span(), "unfinished if")?,
        State::IfMaybeDone(branches) => {
            res.push(parse_if(branches, None)?);
        }
        State::Else(kw, _) => error_at(kw.span(), "unfinished else")?,
    }
    Ok(res)
}
