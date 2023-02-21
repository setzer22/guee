use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    ext::IdentExt, parenthesized, parse::Parse, parse2, Expr, LitStr, PathArguments, Token, Type,
};

#[derive(Default, Debug)]
struct BuilderStructAnnotation {
    is_widget: bool,
    skip_new: bool,
    rename_new: Option<String>,
}

impl Parse for BuilderStructAnnotation {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let contents;
        parenthesized!(contents in input);

        let mut ann = BuilderStructAnnotation::default();
        if contents.peek(Ident::peek_any) {
            loop {
                let id = contents.parse::<Ident>()?;
                if id == "widget" {
                    ann.is_widget = true;
                } else if id == "skip_new" {
                    ann.skip_new = true;
                } else if id == "rename_new" {
                    let _eq = contents.parse::<Token![=]>()?;
                    ann.rename_new = Some(contents.parse::<LitStr>()?.value());
                } else {
                    return Err(syn::Error::new(id.span(), "Unsupported annotation: '{id}'"));
                }
                if contents.parse::<Token![,]>().is_err() {
                    break;
                }
            }
        }
        Ok(ann)
    }
}

#[derive(Default, Debug)]
struct BuilderFieldAnnotation {
    is_default: bool,
    skip_setter: bool,
    strip_option: bool,
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
            } else if id == "strip_option" {
                ann.is_default = true;
                ann.strip_option = true;
            } else {
                return Err(syn::Error::new(
                    id.span(),
                    format!("Invalid annotation: {id}"),
                ));
            }
            if contents.parse::<Token![,]>().is_err() {
                break;
            }
        }

        Ok(ann)
    }
}

impl BuilderFieldAnnotation {
    pub fn validate(&self, struct_ann: &BuilderStructAnnotation, span: Span) -> syn::Result<()> {
        if !struct_ann.is_widget && self.strip_option {
            return Err(syn::Error::new(
                span,
                "Callback fields not supported if #[builder(widget)] is not used.",
            ));
        }
        Ok(())
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

    let mut struct_annotation = BuilderStructAnnotation::default();
    for attr in &input.attrs {
        if attr
            .path
            .get_ident()
            .map(|id| id == "builder")
            .unwrap_or(false)
        {
            struct_annotation = parse2::<BuilderStructAnnotation>(attr.tokens.clone())?;
        }
    }

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
        strip_option: bool,
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
                    let span = field.ident.as_ref().expect("Should be a struct").span();
                    let ann: BuilderFieldAnnotation = syn::parse2(attr.tokens)?;
                    ann.validate(&struct_annotation, span)?;
                    if ann.is_default {
                        optional_fields.push(OptionalField {
                            ident: field.ident.take().unwrap(),
                            ty: field.ty,
                            default_expr: ann.default_expr,
                            skip_setter: ann.skip_setter,
                            strip_option: ann.strip_option,
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

    let constructor = if struct_annotation.skip_new {
        quote! {}
    } else {
        let fn_name = if let Some(new_name) = struct_annotation.rename_new {
            let id = format_ident!("{new_name}");
            quote! { #id }
        } else {
            quote! { new }
        };

        quote! {
            pub fn #fn_name(#(#mandatory_field_signatures),*) -> Self {
                Self {
                    #(#mandatory_field_idents),*,
                    #(#default_initializers),*
                }
            }
        }
    };

    let setters = optional_fields
        .iter()
        .map(|opt| {
            let ident = &opt.ident;
            let ty = &opt.ty;
            if opt.skip_setter {
                Ok(quote!())
            } else {
                let docstring =
                    format!(" Sets the `{ident}` for this `{s_ident}` to a custom value.",);

                let ty_expr = if opt.strip_option {
                    let ty = unwrap_typ(ty, ident.span(), "Option")?;
                    quote! { #ty }
                } else {
                    quote! { #ty }
                };

                let setter_expr = if opt.strip_option {
                    quote! { self.#ident = Some(arg); }
                } else {
                    quote! { self.#ident = arg; }
                };

                Ok(quote! {
                    #[doc = #docstring]
                    pub fn #ident(mut self, arg: #ty_expr) -> Self {
                        #setter_expr
                        self
                    }
                })
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let widget_build_fn = if struct_annotation.is_widget {
        quote! {
            pub fn build(self) -> guee::widget::DynWidget {
                guee::widget::DynWidget::new(self)
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        impl #s_ident {
            #constructor
            #(#setters)*
            #widget_build_fn
        }
    })
}
// Given a generic type with a single argument like Option<T>, returns a Type
// with the inner T
#[allow(unused)] // might be useful later
pub fn unwrap_typ<'a>(typ: &'a Type, span: Span, expected: &str) -> syn::Result<&'a Type> {
    if let Type::Path(typepath) = typ {
        if let Some(seg) = typepath.path.segments.first() {
            if seg.ident == expected {
                if let PathArguments::AngleBracketed(bracketed) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(t)) = bracketed.args.iter().next() {
                        return Ok(t);
                    }
                }
            }
        }
    }
    Err(syn::Error::new(
        span,
        format!(
            "Expected {expected}<_>, found {} instead",
            typ.to_token_stream()
        ),
    ))
}
