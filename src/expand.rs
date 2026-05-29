use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::model::{Container, Field, FieldKind};

pub fn expand_to_kdl(container: &Container) -> TokenStream {
    let ident = &container.ident;
    let (impl_generics, ty_generics, where_clause) = container.generics.split_for_impl();

    let pushes = container.fields.iter().map(field_to_tokens);

    quote! {
        #[allow(non_snake_case, dead_code, unused_macros, clippy::cast_possible_wrap, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss, clippy::cast_lossless)]
        const _: () = {
            trait __ToKdlValue {
                fn __to_kdl_value(&self) -> ::kdl::KdlValue;
            }
            macro_rules! __kdl_int_to {
                ($($t:ty),*) => { $(
                    impl __ToKdlValue for $t {
                        fn __to_kdl_value(&self) -> ::kdl::KdlValue {
                            ::kdl::KdlValue::Integer(*self as i128)
                        }
                    }
                )* };
            }
            __kdl_int_to!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
            impl __ToKdlValue for f32 {
                fn __to_kdl_value(&self) -> ::kdl::KdlValue { ::kdl::KdlValue::Float(*self as f64) }
            }
            impl __ToKdlValue for f64 {
                fn __to_kdl_value(&self) -> ::kdl::KdlValue { ::kdl::KdlValue::Float(*self) }
            }
            impl __ToKdlValue for bool {
                fn __to_kdl_value(&self) -> ::kdl::KdlValue { ::kdl::KdlValue::Bool(*self) }
            }
            impl __ToKdlValue for String {
                fn __to_kdl_value(&self) -> ::kdl::KdlValue { ::kdl::KdlValue::String(self.clone()) }
            }
            impl __ToKdlValue for str {
                fn __to_kdl_value(&self) -> ::kdl::KdlValue { ::kdl::KdlValue::String(self.to_string()) }
            }

            impl #impl_generics #ident #ty_generics #where_clause {
                pub fn to_kdl_document(&self) -> ::kdl::KdlDocument {
                    let mut __doc = ::kdl::KdlDocument::new();
                    #(#pushes)*
                    __doc.autoformat();
                    __doc
                }

                pub fn to_kdl_string(&self) -> String {
                    self.to_kdl_document().to_string()
                }
            }
        };
    }
}

fn field_to_tokens(field: &Field) -> TokenStream {
    let ident = &field.ident;
    let name = &field.name;

    match field.kind {
        FieldKind::Scalar => {
            let build = quote! {
                let mut __node = ::kdl::KdlNode::new(#name);
                __node.push(::kdl::KdlEntry::new(__v.__to_kdl_value()));
                __doc.nodes_mut().push(__node);
            };
            if field.optional {
                quote! { if let ::core::option::Option::Some(__v) = &self.#ident { #build } }
            } else {
                quote! { { let __v = &self.#ident; #build } }
            }
        }
        FieldKind::ScalarVec => {
            let build = quote! {
                let mut __node = ::kdl::KdlNode::new(#name);
                for __item in __list.iter() {
                    __node.push(::kdl::KdlEntry::new(__item.__to_kdl_value()));
                }
                __doc.nodes_mut().push(__node);
            };
            if field.optional {
                quote! { if let ::core::option::Option::Some(__list) = &self.#ident { #build } }
            } else {
                quote! { { let __list = &self.#ident; #build } }
            }
        }
        FieldKind::Child => {
            let build = quote! {
                let mut __node = ::kdl::KdlNode::new(#name);
                __node.set_children(__v.to_kdl_document());
                __doc.nodes_mut().push(__node);
            };
            if field.optional {
                quote! { if let ::core::option::Option::Some(__v) = &self.#ident { #build } }
            } else {
                quote! { { let __v = &self.#ident; #build } }
            }
        }
        FieldKind::ChildVec => {
            let build = quote! {
                for __item in __list.iter() {
                    let mut __node = ::kdl::KdlNode::new(#name);
                    __node.set_children(__item.to_kdl_document());
                    __doc.nodes_mut().push(__node);
                }
            };
            if field.optional {
                quote! { if let ::core::option::Option::Some(__list) = &self.#ident { #build } }
            } else {
                quote! { { let __list = &self.#ident; #build } }
            }
        }
    }
}

pub fn expand_from_kdl(container: &Container) -> TokenStream {
    let ident = &container.ident;
    let (impl_generics, ty_generics, where_clause) = container.generics.split_for_impl();

    let field_lets = container.fields.iter().map(|field| {
        let tmp = format_ident!("__kdl_field_{}", field.ident);
        let expr = field_from_expr(field);
        quote! { let #tmp = #expr; }
    });
    let field_assigns = container.fields.iter().map(|field| {
        let ident = &field.ident;
        let tmp = format_ident!("__kdl_field_{}", field.ident);
        quote! { #ident: #tmp.unwrap() }
    });

    quote! {
        #[allow(non_snake_case, dead_code, unused_macros)]
        const _: () = {
            trait __FromKdlValue: Sized {
                fn __from_kdl_value(__value: &::kdl::KdlValue) -> ::core::result::Result<Self, String>;
            }
            macro_rules! __kdl_int_from {
                ($($t:ty),*) => { $(
                    impl __FromKdlValue for $t {
                        fn __from_kdl_value(__value: &::kdl::KdlValue) -> ::core::result::Result<Self, String> {
                            let __n = __value.as_integer()
                                .ok_or_else(|| format!("expected an integer, found {}", __value))?;
                            <$t as ::core::convert::TryFrom<i128>>::try_from(__n)
                                .map_err(|_| format!(
                                    "integer {} is out of range for `{}`", __n, stringify!($t)
                                ))
                        }
                    }
                )* };
            }
            __kdl_int_from!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
            impl __FromKdlValue for f32 {
                fn __from_kdl_value(__value: &::kdl::KdlValue) -> ::core::result::Result<Self, String> {
                    __value.as_float().map(|__f| __f as f32)
                        .ok_or_else(|| format!("expected a float, found {}", __value))
                }
            }
            impl __FromKdlValue for f64 {
                fn __from_kdl_value(__value: &::kdl::KdlValue) -> ::core::result::Result<Self, String> {
                    __value.as_float().ok_or_else(|| format!("expected a float, found {}", __value))
                }
            }
            impl __FromKdlValue for bool {
                fn __from_kdl_value(__value: &::kdl::KdlValue) -> ::core::result::Result<Self, String> {
                    __value.as_bool().ok_or_else(|| format!("expected a boolean, found {}", __value))
                }
            }
            impl __FromKdlValue for String {
                fn __from_kdl_value(__value: &::kdl::KdlValue) -> ::core::result::Result<Self, String> {
                    __value.as_string().map(|__s| __s.to_string())
                        .ok_or_else(|| format!("expected a string, found {}", __value))
                }
            }

            impl #impl_generics #ident #ty_generics #where_clause {
                pub fn from_kdl_document(
                    __doc: &::kdl::KdlDocument,
                ) -> ::core::result::Result<Self, ::kdl::KdlError> {
                    let __source = ::std::sync::Arc::new(__doc.to_string());
                    Self::__from_kdl_in(__doc, &__source).map_err(|__diagnostics| {
                        ::kdl::KdlError { input: __source, diagnostics: __diagnostics }
                    })
                }

                pub fn from_kdl_str(__src: &str) -> ::core::result::Result<Self, ::kdl::KdlError> {
                    let __doc = ::kdl::KdlDocument::parse(__src)?;
                    let __source = ::std::sync::Arc::new(__src.to_string());
                    Self::__from_kdl_in(&__doc, &__source).map_err(|__diagnostics| {
                        ::kdl::KdlError { input: __source, diagnostics: __diagnostics }
                    })
                }

                #[doc(hidden)]
                pub fn __from_kdl_in(
                    __doc: &::kdl::KdlDocument,
                    __source: &::std::sync::Arc<String>,
                ) -> ::core::result::Result<Self, ::std::vec::Vec<::kdl::KdlDiagnostic>> {
                    let mut __diags: ::std::vec::Vec<::kdl::KdlDiagnostic> = ::std::vec::Vec::new();
                    macro_rules! __push {
                        ($span:expr, $msg:expr) => {
                            __diags.push(::kdl::KdlDiagnostic {
                                input: ::std::sync::Arc::clone(__source),
                                span: $span,
                                message: ::core::option::Option::Some($msg),
                                label: ::core::option::Option::None,
                                help: ::core::option::Option::None,
                                severity: ::core::default::Default::default(),
                            });
                        };
                    }

                    #(#field_lets)*

                    if !__diags.is_empty() {
                        return ::core::result::Result::Err(__diags);
                    }
                    ::core::result::Result::Ok(Self {
                        #(#field_assigns),*
                    })
                }
            }
        };
    }
}

fn field_from_expr(field: &Field) -> TokenStream {
    let name = &field.name;
    let inner_ty = &field.inner_ty;
    let optional = field.optional;

    match field.kind {
        FieldKind::Scalar => {
            let ok = if optional {
                quote! { ::core::option::Option::Some(::core::option::Option::Some(__v)) }
            } else {
                quote! { ::core::option::Option::Some(__v) }
            };
            let missing_node = if optional {
                quote! { ::core::option::Option::Some(::core::option::Option::None) }
            } else {
                quote! {
                    __push!(__doc.span(), format!("missing required node `{}`", #name));
                    ::core::option::Option::None
                }
            };
            quote! {
                match __doc.get(#name) {
                    ::core::option::Option::Some(__node) => {
                        match __node.entries().iter().find(|__e| __e.name().is_none()) {
                            ::core::option::Option::Some(__entry) => {
                                match <#inner_ty as __FromKdlValue>::__from_kdl_value(__entry.value()) {
                                    ::core::result::Result::Ok(__v) => { #ok }
                                    ::core::result::Result::Err(__msg) => {
                                        __push!(__entry.span(), __msg);
                                        ::core::option::Option::None
                                    }
                                }
                            }
                            ::core::option::Option::None => {
                                __push!(__node.span(), format!("node `{}` is missing an argument", #name));
                                ::core::option::Option::None
                            }
                        }
                    }
                    ::core::option::Option::None => { #missing_node }
                }
            }
        }
        FieldKind::ScalarVec => {
            let collect = quote! {
                let mut __out = ::std::vec::Vec::new();
                let mut __ok = true;
                for __e in __node.entries() {
                    if __e.name().is_none() {
                        match <#inner_ty as __FromKdlValue>::__from_kdl_value(__e.value()) {
                            ::core::result::Result::Ok(__v) => __out.push(__v),
                            ::core::result::Result::Err(__msg) => {
                                __push!(__e.span(), __msg);
                                __ok = false;
                            }
                        }
                    }
                }
            };
            if optional {
                quote! {
                    match __doc.get(#name) {
                        ::core::option::Option::Some(__node) => {
                            #collect
                            if __ok {
                                ::core::option::Option::Some(::core::option::Option::Some(__out))
                            } else {
                                ::core::option::Option::None
                            }
                        }
                        ::core::option::Option::None => {
                            ::core::option::Option::Some(::core::option::Option::None)
                        }
                    }
                }
            } else {
                quote! {
                    match __doc.get(#name) {
                        ::core::option::Option::Some(__node) => {
                            #collect
                            if __ok { ::core::option::Option::Some(__out) } else { ::core::option::Option::None }
                        }
                        ::core::option::Option::None => {
                            ::core::option::Option::Some(::std::vec::Vec::new())
                        }
                    }
                }
            }
        }
        FieldKind::Child => {
            let ok = if optional {
                quote! { ::core::option::Option::Some(::core::option::Option::Some(__v)) }
            } else {
                quote! { ::core::option::Option::Some(__v) }
            };
            let missing_node = if optional {
                quote! { ::core::option::Option::Some(::core::option::Option::None) }
            } else {
                quote! {
                    __push!(__doc.span(), format!("missing required node `{}`", #name));
                    ::core::option::Option::None
                }
            };
            quote! {
                match __doc.get(#name) {
                    ::core::option::Option::Some(__node) => {
                        match __node.children() {
                            ::core::option::Option::Some(__children) => {
                                match <#inner_ty>::__from_kdl_in(__children, __source) {
                                    ::core::result::Result::Ok(__v) => { #ok }
                                    ::core::result::Result::Err(mut __ds) => {
                                        __diags.append(&mut __ds);
                                        ::core::option::Option::None
                                    }
                                }
                            }
                            ::core::option::Option::None => {
                                __push!(__node.span(), format!("node `{}` is missing a children block", #name));
                                ::core::option::Option::None
                            }
                        }
                    }
                    ::core::option::Option::None => { #missing_node }
                }
            }
        }
        FieldKind::ChildVec => {
            let collect = quote! {
                let mut __out = ::std::vec::Vec::new();
                let mut __ok = true;
                let mut __found = false;
                for __node in __doc.nodes() {
                    if __node.name().value() == #name {
                        __found = true;
                        match __node.children() {
                            ::core::option::Option::Some(__children) => {
                                match <#inner_ty>::__from_kdl_in(__children, __source) {
                                    ::core::result::Result::Ok(__v) => __out.push(__v),
                                    ::core::result::Result::Err(mut __ds) => {
                                        __diags.append(&mut __ds);
                                        __ok = false;
                                    }
                                }
                            }
                            ::core::option::Option::None => {
                                __push!(__node.span(), format!("node `{}` is missing a children block", #name));
                                __ok = false;
                            }
                        }
                    }
                }
            };
            if optional {
                quote! {
                    {
                        #collect
                        if !__ok {
                            ::core::option::Option::None
                        } else if __found {
                            ::core::option::Option::Some(::core::option::Option::Some(__out))
                        } else {
                            ::core::option::Option::Some(::core::option::Option::None)
                        }
                    }
                }
            } else {
                quote! {
                    {
                        #collect
                        let _ = __found;
                        if __ok { ::core::option::Option::Some(__out) } else { ::core::option::Option::None }
                    }
                }
            }
        }
    }
}
