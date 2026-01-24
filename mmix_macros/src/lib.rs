use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, LitStr, Result, Token, Visibility, braced};

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
    let docs: Vec<Option<&LitStr>> = reg_defs.iter().map(|r| r.doc.as_ref()).collect();
    let count = variants.len();
    let reg_names: Vec<String> = reg_idents.iter().map(|id| id.to_string()).collect();

    let mut enum_name = name.to_string();
    if enum_name.ends_with('s') {
        enum_name.pop();
    }
    if enum_name.is_empty() {
        enum_name = "SpecialRegister".into();
    }

    let enum_ident = Ident::new(&enum_name, name.span());
    let count_ident = format_ident!("{}_COUNT", enum_name.to_uppercase());

    let variant_docs = docs.iter().map(|doc_opt| {
        if let Some(doc) = doc_opt {
            quote! { #[doc = #doc] }
        } else {
            quote! {}
        }
    });

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

        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut dbg = f.debug_struct(stringify!(#name));
                #( dbg.field(#reg_names, &format_args!("{:#018x}", self.#reg_idents)); )*
                dbg.finish()
            }
        }

        pub const #count_ident: usize = #count;
    };

    expanded.into()
}
