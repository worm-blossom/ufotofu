#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::{Block, Expr, Ident, Pat, Token, braced, parse_macro_input};

struct Consume {
    producer: Expr,
    item_case: ConsumeCase<false>,
    final_case: Option<ConsumeCase<true>>,
    error_case: Option<ConsumeCase<true>>,
}

struct ConsumeCase<const SUPPORT_BREAK: bool> {
    pat: Pat,
    code: BlockOrExpr<SUPPORT_BREAK>,
}

impl<const SUPPORT_BREAK: bool> ConsumeCase<SUPPORT_BREAK> {
    fn parse_with_kw<Keyword: Parse>(input: ParseStream) -> Result<Self> {
        input.parse::<Keyword>()?;
        let pat = Pat::parse_single(input)?;
        input.parse::<Token![=>]>()?;
        let code: BlockOrExpr<SUPPORT_BREAK> = input.parse()?;

        Ok(Self { pat, code })
    }
}

enum BlockOrExpr<const SUPPORT_BREAK: bool> {
    Block(Block),
    Expr(Expr),
}

impl<const SUPPORT_BREAK: bool> Parse for BlockOrExpr<SUPPORT_BREAK> {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::token::Brace) {
            Ok(BlockOrExpr::Block(input.parse()?))
        } else {
            let ret = BlockOrExpr::Expr(input.parse()?);
            input.parse::<Token![,]>()?;
            Ok(ret)
        }
    }
}

impl<const SUPPORT_BREAK: bool> quote::ToTokens for BlockOrExpr<SUPPORT_BREAK> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            BlockOrExpr::Block(block) => tokens.extend(quote! {#block}),
            BlockOrExpr::Expr(expr) => {
                tokens.extend(quote! {#expr});
                if SUPPORT_BREAK {
                    tokens.extend(quote! {;})
                } else {
                    tokens.extend(quote! {,})
                }
            }
        }
    }
}

mod kw {
    syn::custom_keyword!(item);
    syn::custom_keyword!(error);
}

impl Parse for Consume {
    fn parse(input: ParseStream) -> Result<Self> {
        let producer: Expr = Expr::parse_without_eager_brace(input)?;

        let cases;
        braced!(cases in input);

        let mut item_case = None;
        let mut final_case = None;
        let mut error_case = None;

        try_parse_case(&cases, &mut item_case, &mut final_case, &mut error_case)?;
        try_parse_case(&cases, &mut item_case, &mut final_case, &mut error_case)?;
        try_parse_case(&cases, &mut item_case, &mut final_case, &mut error_case)?;

        fn try_parse_case(
            cases: &syn::parse::ParseBuffer<'_>,
            item_case: &mut Option<ConsumeCase<false>>,
            final_case: &mut Option<ConsumeCase<true>>,
            error_case: &mut Option<ConsumeCase<true>>,
        ) -> Result<()> {
            let lookahead = cases.lookahead1();
            if lookahead.peek(kw::item) && item_case.is_none() {
                *item_case = Some(ConsumeCase::parse_with_kw::<kw::item>(&cases)?);
            } else if lookahead.peek(Token![final]) && final_case.is_none() {
                *final_case = Some(ConsumeCase::parse_with_kw::<Token![final]>(&cases)?);
            } else if lookahead.peek(kw::error) && error_case.is_none() {
                *error_case = Some(ConsumeCase::parse_with_kw::<kw::error>(&cases)?);
            }

            Ok(())
        }

        match item_case {
            None => panic!("A `consume` macro must contain a case handling items."),
            Some(item_case) => Ok(Consume {
                producer,
                item_case,
                final_case,
                error_case,
            }),
        }
    }
}

#[proc_macro]
pub fn consume(input: TokenStream) -> TokenStream {
    let Consume {
        producer,
        item_case,
        final_case,
        error_case,
    } = parse_macro_input!(input as Consume);

    let ufotofu_crate = get_ufotofu_crate();

    let item = item_case.pat;
    let item_code = item_case.code;

    let expanded = match (final_case, error_case) {
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
        (
            Some(ConsumeCase {
                pat: fin,
                code: fin_code,
            }),
            None,
        ) => {
            quote! {
                loop {
                    match #ufotofu_crate::Producer::produce(#producer).await? {
                        #ufotofu_crate::Either::Left(#item) => #item_code
                        #ufotofu_crate::Either::Right(#fin) => {#[allow(unused)] break #fin_code}
                    }
                }
            }
        }
        (
            None,
            Some(ConsumeCase {
                pat: err,
                code: err_code,
            }),
        ) => {
            quote! {
                loop {
                    match #ufotofu_crate::Producer::produce(#producer).await {
                        core::result::Result::Ok(#ufotofu_crate::Either::Left(#item)) => #item_code
                        core::result::Result::Ok(#ufotofu_crate::Either::Right(())) => break,
                        core::result::Result::Err(#err) => {#[allow(unused)] break #err_code}
                    }
                }
            }
        }
        (
            Some(ConsumeCase {
                pat: fin,
                code: fin_code,
            }),
            Some(ConsumeCase {
                pat: err,
                code: err_code,
            }),
        ) => {
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
    };

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
