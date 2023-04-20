#[macro_export]
macro_rules! unwrap_value {
    ($field_name:ident, Option<$field_type:ty>, $field_id:literal) => {};
    ($field_name:ident, $field_type:ty, $field_id:literal, $default:expr) => {
        let $field_name = $field_name.unwrap_or($default);
    };
    ($field_name:ident, $field_type:ty, $field_id:literal) => {
        let $field_name = $crate::ebml::get_required($field_name, $field_id)?;
    };
}

#[macro_export]
macro_rules! unwrap_parser {
    ($field_id:literal, Option<$field_type:ty>) => {
        $crate::unwrap_parser!($field_id, $field_type)
    };
    ($field_id:literal, Vec<$field_type:ty>) => {
        nom::multi::many0($crate::unwrap_parser!($field_id, $field_type))
    };
    ($field_id:literal, $field_type:ty) => {
        $crate::ebml::ebml_element::<$field_type>($field_id)
    };
}

#[macro_export]
macro_rules! impl_ebml_master {
    (
        $(#[$outer:meta])*
        struct $name:ident$(<$lifetime:lifetime>)? {
            $([$field_id:literal] $field_name:ident: ($($field_type:tt)+) $(= $default:expr)?,)+
        }
    ) => {
        $(#[$outer])*
        pub struct $name$(<$lifetime>)? {
            $(pub $field_name: $($field_type)+,)+
        }

        impl<'p$(:$lifetime, $lifetime)?> $crate::ebml::EbmlParsable<'p> for $name$(<$lifetime>)? {
            // Master Elements can always have a CRC-32 Element
            fn has_crc() -> bool {
                true
            }

            fn try_parse(input: &'p [u8]) -> Result<Self, $crate::ebml::ErrorKind> {
                #[allow(unused_parens)]
                let (i, ($($field_name),*)) = matroska_permutation((
                    $($crate::unwrap_parser!($field_id, $($field_type)+)),+
                ))(input)
                    .map_err(|e| match e {
                        nom::Err::Failure(e) | nom::Err::Error(e) => e.kind,
                        nom::Err::Incomplete(_) => $crate::ebml::ErrorKind::Nom(nom::error::ErrorKind::Complete),
                    })?;

                if !i.is_empty() {
                    log::warn!("{} unused bytes left after parsing {}", i.len(), stringify!($name));
                }

                $($crate::unwrap_value!($field_name, $($field_type)+, $field_id $(, $default)?);)*

                Ok(
                    Self {
                        $($field_name),*
                    }
                )
            }
        }
    };
}
