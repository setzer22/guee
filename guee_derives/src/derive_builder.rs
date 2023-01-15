use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{bracketed, parenthesized, parse::Parse, Expr, Token, Type};

#[derive(Default, Debug)]
struct BuilderFieldAnnotation {
    is_default: bool,
    skip_setter: bool,
    is_callback: bool,
    default_expr: Option<Expr>,
}

impl Parse for BuilderFieldAnnotation {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut ann = BuilderFieldAnnotation::default();

        let contents;
        parenthesized!(contents in input);

        while !contents.is_empty() {
            let id = contents.parse::<Ident>()?;
            if id == "default" {
                ann.is_default = true;
                if contents.peek(Token!(=)) {
                    let _ = contents.parse::<Token!(=)>().unwrap();
                    let expr = contents.parse::<Expr>()?;
                    ann.default_expr = Some(expr);
                }
            } else if id == "skip" {
                ann.is_default = true;
                ann.skip_setter = true;
            } else if id == "callback" {
                ann.is_default = true;
                ann.is_callback = true;
            } else {
                return Err(syn::Error::new(
                    id.span(),
                    format!("Invalid annotation: {id}"),
                ));
            }
        }

        Ok(ann)
    }
}

pub(crate) fn guee_derive_builder_2(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let s = match input.data {
        syn::Data::Struct(s) => s,
        syn::Data::Enum(_) | syn::Data::Union(_) => {
            return Err(syn::Error::new(
                input.ident.span(),
                "Only structs are supported".to_string(),
            ));
        }
    };
    let s_ident = input.ident;

    #[derive(Debug)]
    struct MandatoryField {
        ident: Ident,
        ty: Type,
    }

    #[derive(Debug)]
    struct OptionalField {
        ident: Ident,
        ty: Type,
        default_expr: Option<Expr>,
        skip_setter: bool,
        is_callback: bool,
    }

    impl OptionalField {
        fn default_expr(&self) -> TokenStream {
            self.default_expr
                .as_ref()
                .map(|x| x.to_token_stream())
                .unwrap_or_else(|| quote!(Default::default()))
        }
    }

    let mut mandatory_fields: Vec<MandatoryField> = vec![];
    let mut optional_fields: Vec<OptionalField> = vec![];

    for mut field in s.fields {
        let builder_attr_count = field
            .attrs
            .iter()
            .filter(|x| x.path.get_ident().map(|x| x == "builder").unwrap_or(false))
            .count();

        #[allow(clippy::comparison_chain)]
        if builder_attr_count > 1 {
            return Err(syn::Error::new(
                field.ident.as_ref().unwrap().span(),
                "More than one occurrence of the builder annotation.".to_string(),
            ));
        } else if builder_attr_count == 1 {
            for attr in field.attrs {
                if attr
                    .path
                    .get_ident()
                    .map(|x| x == "builder")
                    .unwrap_or(false)
                {
                    let ann: BuilderFieldAnnotation = syn::parse2(attr.tokens)?;
                    if ann.is_default {
                        optional_fields.push(OptionalField {
                            ident: field.ident.take().unwrap(),
                            ty: field.ty,
                            default_expr: ann.default_expr,
                            skip_setter: ann.skip_setter,
                            is_callback: ann.is_callback,
                        });
                    } else {
                        mandatory_fields.push(MandatoryField {
                            ident: field.ident.take().unwrap(),
                            ty: field.ty,
                        });
                    }
                    // Only process the first "builder" annotation
                    break;
                }
            }
        } else {
            mandatory_fields.push(MandatoryField {
                ident: field.ident.take().unwrap(),
                ty: field.ty,
            });
        }
    }

    let mandatory_field_signatures = mandatory_fields.iter().map(|mdt| {
        let ident = &mdt.ident;
        let typ = &mdt.ty;
        quote! {
            #ident : #typ
        }
    });

    let mandatory_field_idents = mandatory_fields.iter().map(|mdt| {
        let ident = &mdt.ident;
        quote! { #ident }
    });

    let default_initializers = optional_fields.iter().map(|opt| {
        let ident = &opt.ident;
        let default_expr = opt.default_expr();
        quote! {
            #ident : #default_expr
        }
    });

    let constructor = quote! {
        pub fn new(#(#mandatory_field_signatures),*) -> Self {
            Self {
                #(#mandatory_field_idents),*,
                #(#default_initializers),*
            }
        }
    };

    let setters = optional_fields.iter().map(|opt| {
        let ident = &opt.ident;
        let ty = &opt.ty;
        if opt.skip_setter {
            quote!()
        } else if opt.is_callback {
            let docstring = format!(" Sets the `{}` callback for this `{}`.", ident, s_ident);
            quote! {
                #[doc = #docstring]
                pub fn #ident<F, T>(mut self, f: F) -> Self
                where
                    F: FnOnce(&mut T) + 'static,
                    T: 'static,
                {
                    let cb = guee::callback::Callback::from_fn(f);
                    self.#ident = Some(cb);
                    self
                }
            }
        } else {
            let docstring = format!(
                " Sets the `{}` for this `{}` to a custom value.",
                ident, s_ident
            );

            quote! {
                #[doc = #docstring]
                pub fn #ident(mut self, arg: #ty) -> Self {
                    self.#ident = arg;
                    self
                }
            }
        }
    });

    Ok(quote! {
        impl #s_ident {
            #constructor
            #(#setters)*
            pub fn build(self) -> guee::widget::DynWidget {
                guee::widget::DynWidget::new(self)
            }
        }
    })
}
