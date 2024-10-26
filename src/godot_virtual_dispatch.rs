use proc_macro2::TokenStream;

use quote::{format_ident, quote};
use venial::{Path, PathSegment, TypeExpr};

use crate::util::{self, bail};


pub fn godot_virtual_dispatch(meta: TokenStream, input: TokenStream) -> Result<TokenStream, venial::Error> {

    let input_decl = venial::parse_item(input)?;

    let decl = match input_decl {
        venial::Item::Impl(decl) => decl,
        _ => bail!(
            input_decl,
            "#[godot_virtual_dispatch] can only be applied on impl blocks",
        )?,
    };

    if decl.impl_generic_params.is_some() {
        bail!(
            &decl,
            "#[godot_virtual_dispatch] currently does not support generic parameters",
        )?;
    }

    if decl.self_ty.as_path().is_none() {
        return bail!(decl, "invalid Self type for #[godot_virtual_dispatch] impl");
    };

    if !decl.trait_ty.is_some() {
        return bail!(decl, "#[godot_virtual_dispatch] must be attach to a trait implementation");
    }


    let (class_name, trait_path) = util::validate_trait_impl_virtual(&decl, "godot_virtual_dispatch")?;

    let trait_name = trait_path.clone().as_path().unwrap().segments.last().unwrap().clone().ident;
    let fn_name = format_ident!("__get_trait_{trait_name}");

    let mut wrapper_struct_path = trait_path.as_path().unwrap().segments;
    let mut last: &mut PathSegment = wrapper_struct_path.last_mut().unwrap();
    last.ident = format_ident!("{}_Wrapper", last.ident);
    let path = Path { segments: wrapper_struct_path };
    let wrapper_struct_path = path;

    let godot_api = quote!{
        #[godot_api(secondary)]
        impl #class_name {
            
            #[func]
            #[allow(non_snake_case)]
            pub fn #fn_name(&mut self) -> Gd<#wrapper_struct_path> {
                Gd::from_init_fn(|base| #wrapper_struct_path::real_init(base, Box::new(self.to_gd())))
            }
        }
    };

    let result = quote!{
        #decl

        #godot_api
    };


    Ok(result)
}