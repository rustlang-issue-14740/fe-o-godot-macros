use proc_macro2::{Ident, TokenStream};

use quote::{format_ident, quote, ToTokens};
use venial::{FnParam, TraitMember, TypeExpr};

use crate::util::bail;

pub fn wrap_trait(meta: TokenStream, input: TokenStream) -> Result<TokenStream, venial::Error> {

    let input_decl = venial::parse_item(input)?;

    let decl = match &input_decl {
        venial::Item::Trait(decl) => decl,
        _ => bail!(
            input_decl,
            "#[wrap_trait] can only be applied on trait declarations",
        )?,
    };

    let trait_name = &decl.name;
    
    let wrapper_struct_name = format_ident!("{trait_name}_Wrapper");

    
    let mut trait_fn_impls_1: Vec<TokenStream> = Vec::new();
    let mut trait_fn_impls_2: Vec<TokenStream> = Vec::new();

    for i in &decl.body_items {
        match i {
            TraitMember::AssocFunction(f) => {
                let function_name = &f.name;
                let return_ty = f.return_ty.clone().map_or(quote!{()}, |x| x.to_token_stream());

                let params = &f.params;

                let params_in_call : Vec<Ident> = params.clone().iter().skip(1).map(|(p, _)| -> Ident {
                    match p {
                        FnParam::Typed(p) => {
                            p.name.clone()
                        },
                        _ => panic!("expected Typed")
                    }
                }).collect();

                let _1 = quote! {
                    fn #function_name(#params) -> #return_ty {
                        return self.other.#function_name(#(#params_in_call),*);
                    }
                };
                trait_fn_impls_1.push(_1);

                let _2 = quote! {
                    fn #function_name(#params) -> #return_ty {
                        return self.bind_mut().#function_name(#(#params_in_call),*);
                    }
                };
                trait_fn_impls_2.push(_2);
            },
            _ => bail!(
                i,
                "The wrapped trait may only include functions",
            )?,
        }
    }

    let fn_name = format_ident!("__get_trait_{trait_name}");
    let fn_name_string = fn_name.to_string();

    let access_fn_name = format_ident!("try_get_trait_{trait_name}");

    let extra_impl = quote!{
        #[derive(::godot::prelude::GodotClass)]
        #[class] // note: should be 'no_init', but that breaks hot reload
        pub struct #wrapper_struct_name {
            pub other: Box<dyn #trait_name>
        }
        
        impl ::godot::obj::cap::GodotDefault for #wrapper_struct_name {
        
        }
        
        impl #trait_name for #wrapper_struct_name {

            #(#trait_fn_impls_1)*

        }
        
        impl<T: ::godot::prelude::GodotClass> #trait_name for ::godot::prelude::Gd<T>
        where T : #trait_name,
              T : ::godot::prelude::GodotClass + ::godot::obj::Bounds<Declarer = ::godot::obj::bounds::DeclUser>,
        {
            #(#trait_fn_impls_2)*
        }
        
        pub fn #access_fn_name<T>(node: ::godot::prelude::Gd<T>) -> Option<Box<dyn #trait_name>>
        where T : Inherits<::godot::classes::Node> {
            let mut node: ::godot::prelude::Gd<::godot::classes::Node> = node.upcast();
            if node.has_method(#fn_name_string.into()) {
                let method_result = node.call(#fn_name_string.into(), &[]);
                let wrapped : ::godot::prelude::Gd<#wrapper_struct_name> = method_result.to::<::godot::prelude::Gd<#wrapper_struct_name>>();
                let boxed : Box<dyn #trait_name> = Box::new(wrapped);
                return Some(boxed);
            }
        
            return None;
        }
    };

    let result = quote!{
        #decl

        #extra_impl
        
    };

    return Ok(result);
}

