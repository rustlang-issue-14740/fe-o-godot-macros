use proc_macro2::{Ident, TokenStream};

use quote::{format_ident, quote, ToTokens};
use venial::{FnParam, FnReceiverParam, TraitMember, TypeExpr};

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
    let dummy_struct_name = format_ident!("{trait_name}_Dummy");

    
    let mut trait_fn_impls_1: Vec<TokenStream> = Vec::new();
    let mut trait_fn_impls_2: Vec<TokenStream> = Vec::new();
    let mut trait_fn_impls_3: Vec<TokenStream> = Vec::new();
    let warning_str = format!("Object '{trait_name}' can't be initialized from Godot - the init constructor is only provided so hot-reload doesn't break! see https://github.com/godot-rust/gdext/issues/539");

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

                let bind_or_bind_mut = if let FnParam::Receiver(FnReceiverParam { tk_mut: Some(_), .. }) = &params[0].0 { format_ident!("bind_mut") } else { format_ident!("bind") };

                let _2 = quote! {
                    fn #function_name(#params) -> #return_ty {
                        return self.#bind_or_bind_mut().#function_name(#(#params_in_call),*);
                    }
                };
                trait_fn_impls_2.push(_2);


                let _3 = quote! {
                    #[allow(unused)]
                    fn #function_name(#params) -> #return_ty {
                        panic!(#warning_str);
                    }
                };
                trait_fn_impls_3.push(_3);
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
    let access_rec_fn_name = format_ident!("try_get_trait_{trait_name}_rec");

    let wrapper_struct_name_as_string = wrapper_struct_name.to_string();

    let extra_impl = quote!{
        #[derive(::godot::prelude::GodotClass)]
        #[class(base=RefCounted)] // note: should be 'no_init', but that breaks hot reload
        #[allow(non_camel_case_types)]
        pub struct #wrapper_struct_name {
            pub other: Box<dyn #trait_name>,
            base: godot::obj::Base<godot::classes::RefCounted>
        }

        //impl Drop for #wrapper_struct_name {
        //    fn drop(&mut self) {
        //        godot_print!("{} called 'drop'", #wrapper_struct_name_as_string);
        //    }
        //}

        #[allow(non_camel_case_types)]
        struct #dummy_struct_name {

        }

        impl #trait_name for #dummy_struct_name {
            #(#trait_fn_impls_3)*
        }

        #[::godot::prelude::godot_api]
        impl ::godot::prelude::IRefCounted for #wrapper_struct_name {
            fn init(base: ::godot::obj::Base<godot::classes::RefCounted>) -> #wrapper_struct_name {
                ::godot::global::godot_print!(#warning_str);

                Self {
                    other: Box::new(#dummy_struct_name {}),
                    base
                }
            }
        }

        impl #wrapper_struct_name {
            pub fn real_init(base: ::godot::obj::Base<godot::classes::RefCounted>, other: Box<dyn #trait_name>) -> #wrapper_struct_name {
                Self {
                    other,
                    base
                }
            }
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

        /// Check whether the node implements the trait
        #[allow(non_snake_case)]
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

        /// Check whether the node or one of its children implements the trait
        #[allow(non_snake_case)]
        pub fn #access_rec_fn_name<T>(node: ::godot::prelude::Gd<T>) -> Option<Box<dyn #trait_name>>
        where T : Inherits<::godot::classes::Node> {
            let mut node: ::godot::prelude::Gd<::godot::classes::Node> = node.upcast();
            if node.has_method(#fn_name_string.into()) {
                let method_result = node.call(#fn_name_string.into(), &[]);
                let wrapped : ::godot::prelude::Gd<#wrapper_struct_name> = method_result.to::<::godot::prelude::Gd<#wrapper_struct_name>>();
                let boxed : Box<dyn #trait_name> = Box::new(wrapped);
                return Some(boxed);
            } else {
                // do a breadth first search
                let mut current_level = node.get_children_ex().include_internal(true).done();
                while current_level.len() > 0 {
                    // does a node at this level implement the trait?
                    let found = current_level.iter_shared().find_map(|c| #access_fn_name(c.clone()));
                    if found.is_some() {
                        return found;
                    }
                    // get all children of the next level deeper
                    let prev_level = std::mem::replace(&mut current_level, Array::new());
                    for n in prev_level.iter_shared() {
                        for c in n.get_children_ex().include_internal(true).done().iter_shared() {
                            current_level.push(c);
                        }
                    }
                }
                return None;
            }
        }
    };

    let result = quote!{
        #decl

        #extra_impl
        
    };

    return Ok(result);
}

