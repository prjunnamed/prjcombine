use proc_macro::TokenStream;

mod ast;
mod db;
mod emit;
mod eval;
mod parse;

#[proc_macro]
pub fn target_defs(tokens: TokenStream) -> TokenStream {
    fn run(tokens: TokenStream) -> Result<TokenStream, TokenStream> {
        let ast = parse::parse(tokens)?;
        let db = eval::eval(ast)?;
        Ok(emit::emit(db))
    }

    match run(tokens) {
        Ok(res) => res,
        Err(res) => res,
    }
}
