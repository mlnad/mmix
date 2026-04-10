use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, LitStr, Result, Token, Visibility, braced, parenthesized};

struct RegisterDef {
    name: Ident,
    encoding: LitInt,
    doc: Option<LitStr>,
}

impl Parse for RegisterDef {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let encoding: LitInt = input.parse()?;
        let doc = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse::<LitStr>()?)
        } else {
            None
        };
        Ok(Self {
            name,
            encoding,
            doc,
        })
    }
}

struct SpecialRegistersInput {
    vis: Visibility,
    name: Ident,
    regs: syn::punctuated::Punctuated<RegisterDef, Token![,]>,
}

impl Parse for SpecialRegistersInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let vis: Visibility = input.parse()?;
        input.parse::<Token![struct]>()?;
        let name: Ident = input.parse()?;
        let content;
        braced!(content in input);
        let regs = content.parse_terminated(RegisterDef::parse, Token![,])?;
        if regs.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "at least one register is required",
            ));
        }
        Ok(Self { vis, name, regs })
    }
}

fn to_variant_name(ident: &Ident) -> Ident {
    let mut name = ident.to_string();
    if let Some(first) = name.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    format_ident!("{}", name)
}

#[proc_macro]
pub fn define_special_registers(tokens: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(tokens as SpecialRegistersInput);
    let SpecialRegistersInput { vis, name, regs } = input;
    let reg_defs: Vec<RegisterDef> = regs.into_iter().collect();

    let reg_idents: Vec<&Ident> = reg_defs.iter().map(|r| &r.name).collect();
    let variants: Vec<Ident> = reg_idents.iter().map(|id| to_variant_name(id)).collect();
    let encodings: Vec<&LitInt> = reg_defs.iter().map(|r| &r.encoding).collect();
    let reg_names: Vec<String> = reg_idents.iter().map(|id| id.to_string()).collect();
    let count = reg_defs.len();

    let variant_docs: Vec<_> = reg_defs.iter().map(|r| {
        r.doc.as_ref().map(|doc| quote! { #[doc = #doc] })
    }).collect();

    let enum_name = name.to_string().strip_suffix('s')
        .filter(|s| !s.is_empty())
        .map_or("SpecialRegister".to_owned(), str::to_owned);

    let enum_ident = Ident::new(&enum_name, name.span());

    let expanded = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #vis enum #enum_ident {
            #(
                #variant_docs
                #variants
            ),*
        }

        impl #enum_ident {
            pub const ALL: [#enum_ident; #count] = [ #( #enum_ident::#variants ),* ];

            pub const COUNT: usize = #count;

            pub fn name(self) -> &'static str {
                match self {
                    #( #enum_ident::#variants => #reg_names ),*
                }
            }

            pub fn encoding(self) -> u8 {
                match self {
                    #( #enum_ident::#variants => #encodings ),*
                }
            }

            pub fn from_encoding(enc: u8) -> Option<Self> {
                match enc {
                    #( #encodings => Some(#enum_ident::#variants), )*
                    _ => None,
                }
            }
        }

        #[derive(Clone)]
        #vis struct #name {
            #( #reg_idents: u64 ),*
        }

        impl #name {
            pub fn new() -> Self {
                Self { #( #reg_idents: 0 ),* }
            }

            pub fn get(&self, reg: #enum_ident) -> u64 {
                match reg {
                    #( #enum_ident::#variants => self.#reg_idents ),*
                }
            }

            pub fn set(&mut self, reg: #enum_ident, value: u64) {
                match reg {
                    #( #enum_ident::#variants => self.#reg_idents = value ),*
                }
            }

            pub fn iter(&self) -> impl Iterator<Item = (#enum_ident, u64)> + '_ {
                #enum_ident::ALL.into_iter().map(move |r| (r, self.get(r)))
            }
        }

        impl Default for #name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl core::ops::Index<#enum_ident> for #name {
            type Output = u64;
            fn index(&self, reg: #enum_ident) -> &u64 {
                match reg {
                    #( #enum_ident::#variants => &self.#reg_idents ),*
                }
            }
        }

        impl core::ops::IndexMut<#enum_ident> for #name {
            fn index_mut(&mut self, reg: #enum_ident) -> &mut u64 {
                match reg {
                    #( #enum_ident::#variants => &mut self.#reg_idents ),*
                }
            }
        }

        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut dbg = f.debug_struct(stringify!(#name));
                #( dbg.field(#reg_names, &format_args!("{:#018x}", self.#reg_idents)); )*
                dbg.finish()
            }
        }
    };

    expanded.into()
}

// ── define_opcodes! ──────────────────────────────────────────────────────

/// A single opcode entry: `NAME(v, mu)`
struct OpcodeDef {
    name: Ident,
    v: LitInt,
    mu: LitInt,
}

impl Parse for OpcodeDef {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let v: LitInt = content.parse()?;
        content.parse::<Token![,]>()?;
        let mu: LitInt = content.parse()?;
        Ok(Self { name, v, mu })
    }
}

/// Output configuration block: `output { timing: IDENT, names: IDENT, ops: IDENT, }`
struct OutputConfig {
    timing_name: Ident,
    name_table_name: Ident,
    op_mod_name: Ident,
}

impl Parse for OutputConfig {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        braced!(content in input);

        let mut timing_name: Option<Ident> = None;
        let mut name_table_name: Option<Ident> = None;
        let mut op_mod_name: Option<Ident> = None;

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let value: Ident = content.parse()?;
            let _ = content.parse::<Token![,]>();

            match key.to_string().as_str() {
                "timing" => timing_name = Some(value),
                "names" => name_table_name = Some(value),
                "ops" => op_mod_name = Some(value),
                _ => return Err(syn::Error::new(key.span(), format!("unknown output key `{key}`, expected `timing`, `names`, or `ops`"))),
            }
        }

        let timing_name = timing_name.ok_or_else(|| syn::Error::new(Span::call_site(), "missing `timing` in output block"))?;
        let name_table_name = name_table_name.ok_or_else(|| syn::Error::new(Span::call_site(), "missing `names` in output block"))?;
        let op_mod_name = op_mod_name.ok_or_else(|| syn::Error::new(Span::call_site(), "missing `ops` in output block"))?;

        Ok(Self { timing_name, name_table_name, op_mod_name })
    }
}

struct OpcodesInput {
    output: OutputConfig,
    entries: Vec<OpcodeDef>,
}

impl Parse for OpcodesInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // Parse `output { ... }` block
        let kw: Ident = input.parse()?;
        if kw != "output" {
            return Err(syn::Error::new(kw.span(), "expected `output` block"));
        }
        let output: OutputConfig = input.parse()?;

        // Parse opcode entries
        let mut entries = Vec::new();
        while !input.is_empty() {
            entries.push(input.parse::<OpcodeDef>()?);
            // Allow optional trailing comma
            let _ = input.parse::<Token![,]>();
        }
        Ok(Self { output, entries })
    }
}

/// Proc macro that turns a visual 256-entry opcode table into three items
/// whose names are specified by an `output { ... }` configuration block.
///
/// ```ignore
/// mmix_macros::define_opcodes! {
///     output {
///         timing: TIMING_TABLE,
///         names:  NAME_TABLE,
///         ops:    op,
///     }
///     TRAP(5,0), FCMP(1,0), ...
/// }
/// ```
///
/// Generates:
/// - `pub static <timing>: [Timing; 256]`
/// - `pub static <names>: [&str; 256]`
/// - `pub mod <ops> { pub const NAME: u8 = 0xNN; ... }`
///
/// Each entry is `NAME(v, mu)`. The position (0..255) determines the opcode value.
/// Names starting with `_` (e.g. `_2ADDU`) have the `_` stripped in the name table.
#[proc_macro]
pub fn define_opcodes(tokens: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(tokens as OpcodesInput);
    let entries = &input.entries;

    if entries.len() != 256 {
        return syn::Error::new(
            Span::call_site(),
            format!("expected exactly 256 opcode entries, got {}", entries.len()),
        )
        .to_compile_error()
        .into();
    }

    let tn = &input.output.timing_name;
    let nn = &input.output.name_table_name;
    let om = &input.output.op_mod_name;

    // Build timing array entries
    let timing_entries = entries.iter().map(|e| {
        let v = &e.v;
        let mu = &e.mu;
        quote! { Timing::new(#v, #mu) }
    });

    // Build name table entries — strip leading `_` for names like `_2ADDU`
    let name_entries = entries.iter().map(|e| {
        let raw = e.name.to_string();
        let display = raw.strip_prefix('_').unwrap_or(&raw);
        quote! { #display }
    });

    // Build op constants — each entry gets `pub const NAME: u8 = index;`
    let op_consts = entries.iter().enumerate().map(|(i, e)| {
        let name = &e.name;
        let idx = i as u8;
        quote! { pub const #name: u8 = #idx; }
    });

    let expanded = quote! {
        pub static #tn: [Timing; 256] = [
            #( #timing_entries ),*
        ];

        pub static #nn: [&str; 256] = [
            #( #name_entries ),*
        ];

        pub mod #om {
            #( #op_consts )*
        }
    };

    expanded.into()
}
