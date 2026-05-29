use syn::{
    Data, DataStruct, DeriveInput, Expr, Fields, GenericArgument, Ident, LitStr, PathArguments,
    Token, Type,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Scalar,
    ScalarVec,
    Child,
    ChildVec,
}

pub enum DefaultSource {
    None,
    Trait,
    Path(Expr),
}

pub struct Field {
    pub ident: Ident,
    pub name: String,
    pub inner_ty: Type,
    pub optional: bool,
    pub kind: FieldKind,
    pub default: DefaultSource,
}

pub struct Container {
    pub ident: Ident,
    pub generics: syn::Generics,
    pub fields: Vec<Field>,
}

impl Container {
    pub fn from_derive_input(input: &DeriveInput) -> syn::Result<Self> {
        let named = match &input.data {
            Data::Struct(DataStruct {
                fields: Fields::Named(named),
                ..
            }) => named,
            Data::Struct(_) => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "kdl-derive only supports structs with named fields",
                ));
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "kdl-derive can only be derived for structs",
                ));
            }
        };

        let mut fields = Vec::with_capacity(named.named.len());
        for field in &named.named {
            fields.push(Field::parse(field)?);
        }

        Ok(Container {
            ident: input.ident.clone(),
            generics: input.generics.clone(),
            fields,
        })
    }
}

impl Field {
    fn parse(field: &syn::Field) -> syn::Result<Self> {
        let ident = field
            .ident
            .clone()
            .expect("named fields always have an identifier");

        let mut name_override: Option<String> = None;
        let mut is_child = false;
        let mut default = DefaultSource::None;

        for attr in &field.attrs {
            if !attr.path().is_ident("kdl") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("child") {
                    is_child = true;
                    Ok(())
                } else if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let lit: LitStr = value.parse()?;
                    name_override = Some(lit.value());
                    Ok(())
                } else if meta.path.is_ident("default") {
                    if meta.input.peek(Token![=]) {
                        let value = meta.value()?;
                        let lit: LitStr = value.parse()?;
                        default = DefaultSource::Path(lit.parse()?);
                    } else {
                        default = DefaultSource::Trait;
                    }
                    Ok(())
                } else {
                    Err(meta.error(
                        "unknown `kdl` attribute, expected `child`, `name = \"..\"`, \
                         `default` or `default = \"..\"`",
                    ))
                }
            })?;
        }

        let (optional, after_option) = match inner_of(&field.ty, "Option") {
            Some(inner) => (true, inner),
            None => (false, field.ty.clone()),
        };
        let (is_vec, inner_ty) = match inner_of(&after_option, "Vec") {
            Some(inner) => (true, inner),
            None => (false, after_option),
        };

        let kind = match (is_child, is_vec) {
            (true, true) => FieldKind::ChildVec,
            (true, false) => FieldKind::Child,
            (false, true) => FieldKind::ScalarVec,
            (false, false) => FieldKind::Scalar,
        };

        let name = name_override.unwrap_or_else(|| ident.to_string());

        Ok(Field {
            ident,
            name,
            inner_ty,
            optional,
            kind,
            default,
        })
    }
}

fn inner_of(ty: &Type, wrapper: &str) -> Option<Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != wrapper {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(inner) => Some(inner.clone()),
        _ => None,
    })
}
