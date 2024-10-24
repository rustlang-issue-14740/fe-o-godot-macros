use util::bail;



mod wrap_trait;
mod godot_virtual_dispatch;
mod util;


use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
/// apply this to a trait
#[proc_macro_attribute]
pub fn wrap_trait(meta: TokenStream, input: TokenStream) -> TokenStream {
    let result = crate::wrap_trait::wrap_trait(TokenStream2::from(meta), TokenStream2::from(input));
    match result {
        Ok(ts) => TokenStream::from(ts),
        Err(e) => TokenStream::from(e.to_compile_error())
    }
}


/// apply this to an 'impl Trait for Class' block of a Godot Custom Class
#[proc_macro_attribute]
pub fn godot_virtual_dispatch(meta: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let result = crate::godot_virtual_dispatch::godot_virtual_dispatch(TokenStream2::from(meta), TokenStream2::from(input));
    match result {
        Ok(ts) => TokenStream::from(ts),
        Err(e) => TokenStream::from(e.to_compile_error())
    }
}