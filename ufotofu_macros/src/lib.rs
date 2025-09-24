#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::{Block, Expr, Ident, Pat, Token, parenthesized, parse_macro_input};

struct Consume {
    item: Pat,
    producer: Expr,
    item_code: Block,
    final_block: Option<FinalBlock>,
    catch_block: Option<CatchBlock>,
}

impl Parse for Consume {
    fn parse(input: ParseStream) -> Result<Self> {
        let item: Pat = Pat::parse_single(input)?;
        input.parse::<Token![in]>()?;
        let producer: Expr = Expr::parse_without_eager_brace(input)?;
        let item_code: Block = input.parse()?;

        let mut final_block = None;
        let mut catch_block = None;

        let lookahead = input.lookahead1();
        if lookahead.peek(Token![final]) {
            final_block = Some(input.parse()?);
        } else if lookahead.peek(kw::catch) {
            catch_block = Some(input.parse()?);
        }

        if lookahead.peek(Token![final]) && final_block.is_none() {
            final_block = Some(input.parse()?);
        } else if lookahead.peek(kw::catch) && catch_block.is_none() {
            catch_block = Some(input.parse()?);
        }

        Ok(Consume {
            item,
            producer,
            item_code,
            final_block,
            catch_block,
        })
    }
}

struct FinalBlock {
    fin: Pat,
    fin_code: Block,
}

impl Parse for FinalBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![final]>()?;
        let content;
        parenthesized!(content in input);
        let fin: Pat = Pat::parse_single(&content)?;
        let fin_code: Block = input.parse()?;

        Ok(FinalBlock { fin, fin_code })
    }
}

struct CatchBlock {
    err: Pat,
    err_code: Block,
}

mod kw {
    syn::custom_keyword!(catch);
}

impl Parse for CatchBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<kw::catch>()?;
        // let err: Pat = Pat::parse_single(input)?;
        let content;
        parenthesized!(content in input);
        let err: Pat = Pat::parse_single(&content)?;
        let err_code: Block = input.parse()?;

        Ok(CatchBlock { err, err_code })
    }
}

#[proc_macro]
pub fn consume(input: TokenStream) -> TokenStream {
    let Consume {
        item,
        producer,
        item_code,
        final_block,
        catch_block,
    } = parse_macro_input!(input as Consume);

    let ufotofu_crate = get_ufotofu_crate();

    let expanded = match (final_block, catch_block) {
        (None, None) => {
            quote! {
                loop {
                    match #ufotofu_crate::Producer::produce(#producer).await? {
                        #ufotofu_crate::Either::Left(#item) => #item_code
                        #ufotofu_crate::Either::Right(()) => break,
                    }
                }
            }
        }
        (Some(FinalBlock { fin, fin_code }), Some(CatchBlock { err, err_code })) => {
            quote! {
                loop {
                    match #ufotofu_crate::Producer::produce(#producer).await {
                        core::result::Result::Ok(#ufotofu_crate::Either::Left(#item)) => #item_code
                        core::result::Result::Ok(#ufotofu_crate::Either::Right(#fin)) => {#[allow(unused)] break #fin_code}
                        core::result::Result::Err(#err) => {#[allow(unused)] break #err_code}
                    }
                }
            }
        }
        _ => todo!(),
    };

    let expanded = quote! {42};

    TokenStream::from(expanded)
}

fn get_ufotofu_crate() -> proc_macro2::TokenStream {
    match crate_name("ufotofu").expect("ufotofu is present in `Cargo.toml`") {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
    }
}
