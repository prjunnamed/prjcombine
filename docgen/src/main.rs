use std::{
    collections::{BTreeMap, HashSet},
    io, process,
};

use clap::{Arg, ArgMatches, Command};
use coolrunner2::gen_coolrunner2;
use mdbook::{
    BookItem,
    book::{Book, Chapter},
    errors::{Error, Result},
    preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext},
};
use semver::{Version, VersionReq};
use siliconblue::gen_siliconblue;
use spartan6::gen_spartan6;
use virtex::gen_virtex;
use virtex2::gen_virtex2;
use virtex4::gen_virtex4;
use xc2000::gen_xc2000;
use xc9500::gen_xc9500;
use xpla3::gen_xpla3;

use crate::{ecp::gen_ecp, ultrascale::gen_ultrascale};

mod bsdata;
mod coolrunner2;
mod ecp;
mod interconnect;
mod siliconblue;
mod spartan6;
mod speed;
mod ultrascale;
mod virtex;
mod virtex2;
mod virtex4;
mod xc2000;
mod xc9500;
mod xpla3;

pub fn make_app() -> Command {
    Command::new("nop-preprocessor")
        .about("A mdbook preprocessor which does precisely nothing")
        .subcommand(
            Command::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

fn main() {
    let matches = make_app().get_matches();

    let preprocessor = Docgen;

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    let book_version = Version::parse(&ctx.mdbook_version)?;
    let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

    if !version_req.matches(&book_version) {
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args
        .get_one::<String>("renderer")
        .expect("Required argument");
    let supported = pre.supports_renderer(renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

struct Docgen;

struct DocgenContext<'a> {
    ctx: &'a PreprocessorContext,
    items: BTreeMap<String, String>,
    extra_docs: BTreeMap<String, Vec<(String, String, String)>>,
}

impl Preprocessor for Docgen {
    fn name(&self) -> &str {
        "prjcombine-docgen"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let mut ctx = DocgenContext {
            ctx,
            items: BTreeMap::new(),
            extra_docs: BTreeMap::new(),
        };

        gen_siliconblue(&mut ctx);
        gen_xc2000(&mut ctx);
        gen_virtex(&mut ctx);
        gen_virtex2(&mut ctx);
        gen_spartan6(&mut ctx);
        gen_virtex4(&mut ctx);
        gen_ultrascale(&mut ctx);
        gen_ecp(&mut ctx);

        gen_xc9500(&mut ctx);
        gen_xpla3(&mut ctx);
        gen_coolrunner2(&mut ctx);

        let mut items_used = HashSet::new();

        book.for_each_mut(|section| {
            let BookItem::Chapter(chapter) = section else {
                return;
            };
            if chapter.is_draft_chapter() {
                return;
            }
            let mut new_content = String::new();
            let mut content = chapter.content.as_str();
            while let Some(pos) = content.find("{{") {
                new_content.push_str(&content[..pos]);
                content = &content[pos + 2..];
                let pos = content.find("}}").unwrap();
                let tag = &content[..pos];
                let tag = tag.trim().replace(" ", "-");
                content = &content[pos + 2..];
                new_content.push_str(
                    ctx.items
                        .get(&tag)
                        .unwrap_or_else(|| panic!("no item {tag}")),
                );
                items_used.insert(tag);
            }
            new_content.push_str(content);
            chapter.content = new_content;

            if let Some(ref path) = chapter.path {
                let path = path.to_string_lossy().into_owned();
                if let Some(extras) = ctx.extra_docs.get(&path) {
                    let mut index = 1;
                    let mut parent_names = chapter.parent_names.clone();
                    parent_names.push(chapter.name.clone());
                    for (path, name, content) in extras {
                        let mut number = chapter.number.clone();
                        if let Some(ref mut number) = number {
                            number.push(index);
                        }
                        chapter.sub_items.push(BookItem::Chapter(Chapter {
                            name: name.clone(),
                            content: content.clone(),
                            number,
                            sub_items: vec![],
                            path: Some(path.clone().into()),
                            source_path: None,
                            parent_names: parent_names.clone(),
                        }));
                        index += 1;
                    }
                }
            }
        });

        for item in ctx.items.keys() {
            if !items_used.contains(item) {
                eprintln!("WARNING: unused item {item}");
            }
        }

        Ok(book)
    }
}
