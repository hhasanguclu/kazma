//! `tr!("{n} files", "{n} dosya", ...)`: English and Turkish are checked by
//! `format!` at compile time; every other language is looked up at runtime by
//! the English text and filled with the same values (see src/i18n.rs).

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitStr, Token};

struct Input {
    en: LitStr,
    tr: LitStr,
    pos: Vec<Expr>,
    named: Vec<(Ident, Expr)>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let en: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let tr: LitStr = input.parse()?;
        let (mut pos, mut named) = (vec![], vec![]);
        while input.parse::<Option<Token![,]>>()?.is_some() {
            if input.is_empty() {
                break;
            }
            if input.peek(Ident) && input.peek2(Token![=]) && !input.peek2(Token![==]) {
                let name: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                named.push((name, input.parse()?));
            } else {
                pos.push(input.parse()?);
            }
        }
        Ok(Input { en, tr, pos, named })
    }
}

/// One `{...}` in a format string: the argument as written and the spec after ':'.
struct Hole {
    arg: String,
    spec: Option<String>,
}

/// Split a format string into literal text and holes. `{{` / `}}` stay literal.
fn holes(s: &str) -> Result<Vec<Result<String, Hole>>, String> {
    let mut out = vec![];
    let mut lit = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' if it.peek() == Some(&'{') => {
                it.next();
                lit.push_str("{{");
            }
            '}' if it.peek() == Some(&'}') => {
                it.next();
                lit.push_str("}}");
            }
            '{' => {
                let mut inner = String::new();
                loop {
                    match it.next() {
                        Some('}') => break,
                        Some(c) => inner.push(c),
                        None => return Err(format!("unclosed '{{' in {s:?}")),
                    }
                }
                if !lit.is_empty() {
                    out.push(Ok(std::mem::take(&mut lit)));
                }
                let (arg, spec) = match inner.split_once(':') {
                    Some((a, sp)) => (a.trim().to_string(), Some(sp.to_string())),
                    None => (inner.trim().to_string(), None),
                };
                out.push(Err(Hole { arg, spec }));
            }
            c => lit.push(c),
        }
    }
    if !lit.is_empty() {
        out.push(Ok(lit));
    }
    Ok(out)
}

/// The local binding a hole refers to: positional args and named args are bound
/// to `__kz_*` locals first so each expression runs once; inline captures stay.
fn binding(arg: &str, next: &mut usize, named: &[(Ident, Expr)]) -> Result<String, String> {
    if arg.is_empty() {
        *next += 1;
        Ok(format!("__kz_p{}", *next - 1))
    } else if arg.chars().all(|c| c.is_ascii_digit()) {
        Ok(format!("__kz_p{arg}"))
    } else if named.iter().any(|(n, _)| n == arg) {
        Ok(format!("__kz_n_{arg}"))
    } else {
        Ok(arg.to_string())
    }
}

/// Rewrite the spec's `name$` / `0$` width/precision references.
fn rewrite_spec(spec: &str, named: &[(Ident, Expr)]) -> Result<String, String> {
    if spec.contains(".*") {
        return Err("`.*` precision is not supported in tr!".into());
    }
    let mut out = String::new();
    let mut word = String::new();
    for c in spec.chars() {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else if c == '$' && !word.is_empty() {
            let mut dummy = 0;
            out.push_str(&binding(&word, &mut dummy, named)?);
            out.push('$');
            word.clear();
        } else {
            out.push_str(&word);
            word.clear();
            out.push(c);
        }
    }
    out.push_str(&word);
    Ok(out)
}

/// A hole: (key as written in the source, local binding, spec).
type HoleInfo = (String, String, String);

/// The format string with every hole pointing at a named local, plus the holes
/// as (key as written in the source, local, spec) for runtime translation.
fn rewrite(s: &str, named: &[(Ident, Expr)]) -> Result<(String, Vec<HoleInfo>), String> {
    let mut out = String::new();
    let mut list = vec![];
    let mut next = 0;
    for part in holes(s)? {
        match part {
            Ok(text) => out.push_str(&text),
            Err(h) => {
                let b = binding(&h.arg, &mut next, named)?;
                let spec = match &h.spec {
                    Some(sp) => format!(":{}", rewrite_spec(sp, named)?),
                    None => String::new(),
                };
                out.push_str(&format!("{{{b}{spec}}}"));
                list.push((h.arg.clone(), b, spec));
            }
        }
    }
    Ok((out, list))
}

#[proc_macro]
pub fn tr(input: TokenStream) -> TokenStream {
    let Input { en, tr, pos, named } = syn::parse_macro_input!(input as Input);
    let span = Span::call_site();
    let (en_fmt, en_holes) = match rewrite(&en.value(), &named) {
        Ok(v) => v,
        Err(e) => return syn::Error::new(en.span(), e).to_compile_error().into(),
    };
    let (tr_fmt, _) = match rewrite(&tr.value(), &named) {
        Ok(v) => v,
        Err(e) => return syn::Error::new(tr.span(), e).to_compile_error().into(),
    };
    let en_lit = LitStr::new(&en_fmt, span);
    let tr_lit = LitStr::new(&tr_fmt, span);
    let key = LitStr::new(&en.value(), span);
    let pos_names: Vec<Ident> = (0..pos.len())
        .map(|i| Ident::new(&format!("__kz_p{i}"), span))
        .collect();
    let named_names: Vec<Ident> = named
        .iter()
        .map(|(n, _)| Ident::new(&format!("__kz_n_{n}"), span))
        .collect();
    let named_exprs: Vec<&Expr> = named.iter().map(|(_, e)| e).collect();
    let values = en_holes.iter().map(|(k, b, spec)| {
        let one = LitStr::new(&format!("{{{b}{spec}}}"), span);
        quote! { (#k, format!(#one)) }
    });
    // A match keeps temporaries in the arguments alive, like format_args! does.
    quote! {
        match (#( &(#pos), )* #( &(#named_exprs), )*) {
            (#( #pos_names, )* #( #named_names, )*) => match crate::i18n::lang() {
                crate::i18n::Lang::En => format!(#en_lit),
                crate::i18n::Lang::Tr => format!(#tr_lit),
                _ => crate::i18n::render(#key, &[#(#values),*]).unwrap_or_else(|| format!(#en_lit)),
            },
        }
    }
    .into()
}
