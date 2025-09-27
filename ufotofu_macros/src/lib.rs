#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenTree};
use quote::{TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream, Result};
use syn::{Block, Expr, Ident, Pat, Token, braced, parse_macro_input};

struct Consume {
    producer: Expr,
    item_cases: Vec<ConsumeCase>,
    final_cases: Vec<ConsumeCase>,
    error_cases: Vec<ConsumeCase>,
}

struct ConsumeCase {
    pat: Pat,
    code: BlockOrExpr,
}

impl ConsumeCase {
    fn parse_with_kw<Keyword: Parse>(input: ParseStream) -> Result<Self> {
        input.parse::<Keyword>()?;
        let pat = Pat::parse_single(input)?;
        input.parse::<Token![=>]>()?;
        let code: BlockOrExpr = input.parse()?;

        Ok(Self { pat, code })
    }
}

enum BlockOrExpr {
    Block(Block),
    Expr(Expr),
}

impl Parse for BlockOrExpr {
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

impl BlockOrExpr {
    fn to_tokens_maybe_break_support(&self, support_break: bool) -> proc_macro2::TokenStream {
        match self {
            BlockOrExpr::Block(block) => quote! {{#[allow(unused)] break #block}},
            BlockOrExpr::Expr(expr) => {
                if support_break {
                    quote! {{#[allow(unused)] break #expr ;}}
                } else {
                    quote! {#expr ,}
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

        let mut item_cases = vec![];
        let mut final_cases = vec![];
        let mut error_cases = vec![];

        while try_parse_case(&cases, &mut item_cases, &mut final_cases, &mut error_cases)? {}

        if item_cases.is_empty() {
            panic!("A `consume` macro must contain at least one case handling items.");
        } else {
            return Ok(Consume {
                producer,
                item_cases: item_cases,
                final_cases: final_cases,
                error_cases: error_cases,
            });
        }

        fn try_parse_case(
            cases: &syn::parse::ParseBuffer<'_>,
            item_case: &mut Vec<ConsumeCase>,
            final_case: &mut Vec<ConsumeCase>,
            error_case: &mut Vec<ConsumeCase>,
        ) -> Result<bool> {
            let lookahead = cases.lookahead1();
            Ok(if lookahead.peek(kw::item) {
                item_case.push(ConsumeCase::parse_with_kw::<kw::item>(&cases)?);
                true
            } else if lookahead.peek(Token![final]) {
                final_case.push(ConsumeCase::parse_with_kw::<Token![final]>(&cases)?);
                true
            } else if lookahead.peek(kw::error) {
                error_case.push(ConsumeCase::parse_with_kw::<kw::error>(&cases)?);
                true
            } else {
                false
            })
        }
    }
}

struct ItemCase<'a> {
    pat: Pat,
    code: BlockOrExpr,
    in_result: bool,
    ufotofu_crate: &'a proc_macro2::TokenStream,
}

impl<'a> quote::ToTokens for ItemCase<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let pat = &self.pat;
        let rhs = self.code.to_tokens_maybe_break_support(false);
        let ufotofu_crate = self.ufotofu_crate;

        tokens.extend(if self.in_result {
            quote! {
                core::result::Result::Ok(#ufotofu_crate::Either::Left(#pat)) => #rhs
            }
        } else {
            {
                quote! {
                    #ufotofu_crate::Either::Left(#pat) => #rhs
                }
            }
        })
    }
}

struct FinalCase<'a> {
    pat: Pat,
    code: BlockOrExpr,
    in_result: bool,
    ufotofu_crate: &'a proc_macro2::TokenStream,
}

impl<'a> quote::ToTokens for FinalCase<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let pat = &self.pat;
        let rhs = self.code.to_tokens_maybe_break_support(true);
        let ufotofu_crate = self.ufotofu_crate;

        tokens.extend(if self.in_result {
            quote! {
                core::result::Result::Ok(#ufotofu_crate::Either::Right(#pat)) => #rhs
            }
        } else {
            {
                quote! {
                    #ufotofu_crate::Either::Right(#pat) => #rhs
                }
            }
        })
    }
}

struct ErrorCase {
    pat: Pat,
    code: BlockOrExpr,
}

impl quote::ToTokens for ErrorCase {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let pat = &self.pat;
        let rhs = self.code.to_tokens_maybe_break_support(true);

        tokens.extend(quote! {
            core::result::Result::Err(#pat) => #rhs
        })
    }
}

#[proc_macro]
pub fn consume(input: TokenStream) -> TokenStream {
    let Consume {
        producer,
        item_cases,
        final_cases,
        error_cases,
    } = parse_macro_input!(input as Consume);

    let in_result = !error_cases.is_empty();

    let ufotofu_crate = get_ufotofu_crate();

    let item_cases: Vec<_> = item_cases
        .into_iter()
        .map(|consume_case| ItemCase {
            pat: consume_case.pat,
            code: consume_case.code,
            in_result,
            ufotofu_crate: &ufotofu_crate,
        })
        .collect();

    let final_cases: Vec<_> = final_cases
        .into_iter()
        .map(|consume_case| FinalCase {
            pat: consume_case.pat,
            code: consume_case.code,
            in_result,
            ufotofu_crate: &ufotofu_crate,
        })
        .collect();

    let error_cases: Vec<_> = error_cases
        .into_iter()
        .map(|consume_case| ErrorCase {
            pat: consume_case.pat,
            code: consume_case.code,
        })
        .collect();

    let expanded = match (final_cases.is_empty(), error_cases.is_empty()) {
        (true, true) => {
            quote! {
                {
                    let mut  p = #ufotofu_crate::IntoProducer::into_producer(#producer);
                    loop {
                        match #ufotofu_crate::Producer::produce(&mut p).await? {
                            #(#item_cases)*
                            #ufotofu_crate::Either::Right(()) => break,
                        }
                    }
                }
            }
        }
        (false, true) => {
            quote! {
                {
                    let mut  p = #ufotofu_crate::IntoProducer::into_producer(#producer);
                    loop {
                        match #ufotofu_crate::Producer::produce(&mut p).await? {
                            #(#item_cases)*
                            #(#final_cases)*
                        }
                    }
                }
            }
        }
        (true, false) => {
            quote! {
                {
                    let mut  p = #ufotofu_crate::IntoProducer::into_producer(#producer);
                    loop {
                        match #ufotofu_crate::Producer::produce(&mut p).await {
                            #(#item_cases)*
                            core::result::Result::Ok(#ufotofu_crate::Either::Right(())) => break,
                            #(#error_cases)*
                        }
                    }
                }
            }
        }
        (false, false) => {
            quote! {
                {
                    let mut  p = #ufotofu_crate::IntoProducer::into_producer(#producer);
                    loop {
                        match #ufotofu_crate::Producer::produce(&mut p).await {
                            #(#item_cases)*
                            #(#final_cases)*
                            #(#error_cases)*
                        }
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
