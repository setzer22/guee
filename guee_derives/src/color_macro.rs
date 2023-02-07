use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse::Parse, parse2, LitFloat, LitInt, LitStr, Token};

struct ColorSpec([u8; 4]);

pub fn color_from_hex(span: Span, hex: &str) -> syn::Result<[u8; 4]> {
    // Convert a hex string to decimal. Eg. "00" -> 0. "FF" -> 255.
    fn _hex_dec(span: Span, hex_string: &str) -> syn::Result<u8> {
        match u8::from_str_radix(hex_string, 16) {
            Ok(o) => Ok(o),
            Err(e) => Err(syn::Error::new(span, format!("Error parsing hex: {e}"))),
        }
    }

    if !hex.starts_with('#') {
        return Err(syn::Error::new(span, "Hex color should start with #"));
    }

    if hex.len() == 9 && hex.starts_with('#') {
        // #FFFFFFFF (Red Green Blue Alpha)
        return Ok([
            _hex_dec(span, &hex[1..3])?,
            _hex_dec(span, &hex[3..5])?,
            _hex_dec(span, &hex[5..7])?,
            _hex_dec(span, &hex[7..9])?,
        ]);
    } else if hex.len() == 7 && hex.starts_with('#') {
        // #FFFFFF (Red Green Blue)
        return Ok([
            _hex_dec(span, &hex[1..3])?,
            _hex_dec(span, &hex[3..5])?,
            _hex_dec(span, &hex[5..7])?,
            u8::MAX,
        ]);
    }

    Err(syn::Error::new(
        span,
        format!("Error parsing hex: {hex}. Example of valid formats: #FFFFFF or #ffffffff"),
    ))
}

struct Number(u8);

impl Parse for Number {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let float = if let Ok(float_lit) = input.parse::<LitFloat>() {
            float_lit.base10_parse::<f32>()?
        } else if let Ok(int_lit) = input.parse::<LitInt>() {
            int_lit.base10_parse::<u32>()? as f32
        } else {
            return Err(syn::Error::new(input.span(), "Expected a number"));
        };

        if float >= 0.0 && float <= u8::MAX as f32 {
            Ok(Number((float * u8::MAX as f32) as u8))
        } else {
            Err(syn::Error::new(
                input.span(),
                "Number should be between 0 and 255",
            ))
        }
    }
}

impl Parse for ColorSpec {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        // A string literal is interpreted as a hex-formatted string
        if input.peek(LitStr) {
            let str_lit = input.parse::<LitStr>()?;
            color_from_hex(span, &str_lit.value()).map(ColorSpec)
        }
        // A tuple of numbers is interpreted
        else if input.peek(LitFloat) || input.peek(LitInt) {
            let first_number = input.parse::<Number>()?.0;
            let _comma = input.parse::<Token![,]>()?;
            let second_number = input.parse::<Number>()?.0;
            let _comma = input.parse::<Token![,]>()?;
            let third_number = input.parse::<Number>()?.0;
            let fourth_number = if input.peek(Token![,]) {
                let _comma = input.parse::<Token![,]>()?;
                input.parse::<Number>()?.0
            } else {
                255
            };
            Ok(ColorSpec([
                first_number,
                second_number,
                third_number,
                fourth_number,
            ]))
        } else {
            Err(syn::Error::new(
                span,
                "Expected a hex string, or comma-separated list of RGB[A] numbers",
            ))
        }
    }
}

pub fn color_macro2(input: TokenStream) -> syn::Result<TokenStream> {
    let color_spec = parse2::<ColorSpec>(input)?;
    let [r, g, b, a] = color_spec.0;
    Ok(quote! {
        ::epaint::Color32::from_rgba_unmultiplied(#r, #g, #b, #a)
    })
}
