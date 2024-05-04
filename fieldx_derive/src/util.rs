pub(crate) mod args;

#[cfg(not(debug_assertions))]
#[allow(unused)]
macro_rules! TODO {
    ($message:expr; $dummy:expr) => {
        compile_error!(concat!(
            "TODO: Must implement prior to release: ",
            $message
        ));
    };
    ($message:expr;) => {
        compile_error!(concat!(
            "TODO: Must implement prior to release: ",
            $message
        ));
    };
    ($message:expr) => {
        compile_error!(concat!(
            "TODO: Must implement prior to release: ",
            $message
        ));
    };
}

#[cfg(debug_assertions)]
#[allow(unused)]
macro_rules! TODO {
    ($message:expr; $dummy:expr) => {{
        $dummy
    }};
    ($message:expr;) => {{
        unimplemented!($message)
    }};
    ($message:expr) => {{
        unimplemented!($message)
    }};
}

macro_rules! validate_exclusives {
    ($( $group:expr => $( $field:ident ),+ );+) => {
        fn validate_exclusives(&self) -> ::darling::Result<()> {
            $(
                {
                    let mut set_params: Vec<&str> = vec![];
                    $(
                        if self.$field.is_some() {
                            set_params.push(stringify!($field));
                        }
                    )+

                    if set_params.len() > 1 {
                        let err = darling::Error::custom(
                            format!("The following options from group '{}' cannot be used together: {}", $group, set_params.iter().map(|f| format!("`{}`", f)).collect::<Vec<String>>().join(", "))
                        );

                        return Err(err);
                    }
                }
            )+
            Ok(())
        }
    };
}

macro_rules! needs_helper {
    ( $( $field:ident ),+ ) => {
        ::paste::paste!{
            $(
                #[inline]
                pub fn [<needs_ $field>](&self) -> Option<bool> {
                    self.$field.as_ref().map(|h| h.is_true())
                }
            )+
        }
    };
}

macro_rules! set_literals {
    ( $name:ident, $min:literal .. $($max:literal)? => $( $field:ident as $ty:path ),+ $( ; pre_validate => $pre_validate:ident )? ) => {
        fn set_literals(#[allow(unused_mut)] mut self, literals: &Vec<::syn::Lit>) -> ::darling::Result<Self> {
            $( let _: () = self.$pre_validate(literals)?; )?
            #[allow(unused_comparisons)]
            if literals.len() < $min {
                return Err( darling::Error::custom(format!("Too few literal arguments for {}", stringify!($name))) )
            }
            $(
                if literals.len() > $max {
                    return Err( darling::Error::custom(format!("Too many literal arguments for {}", stringify!($name))) )
                }
            )?
            let mut iter = literals.iter();
            $(
                if let Some(lit) = iter.next() {
                    if let $ty(lit_value) = lit {
                        // XXX Well, this only works for a single literal...
                        self.$field = lit_value.value().into();
                    }
                    else {
                        return Err(
                            darling::Error::custom(
                                format!("Expected a {} literal argument for `{}`",
                                    stringify!($ty),
                                    stringify!($field))
                            )
                            .with_span(lit)
                        );
                    }
                }
            )+
            Ok(self)
        }
    };
    ($name:ident, .. $max:tt => $( $field:ident as $ty:path ),+ $( ; pre_validate => $pre_validate:ident )? ) => {
        set_literals! {$name, 0 .. $max => $( $field as $ty ),+ $( ; pre_validate => $pre_validate )?}
    };
    ($name:ident $(, .. 0 )?) => {
        fn set_literals(#[allow(unused_mut)] mut self, _literals: &Vec<::syn::Lit>) -> ::darling::Result<Self> {
            Err(darling::Error::custom(format!("No literals are allowed with `{}`", stringify!($name))))
        }
    };
}

#[cfg(feature = "tracing")]
#[allow(unused_macros)]
macro_rules! fxtrace {
    ( $( $disp:expr ),* ) => {
        eprint!("&&& {}:{}", file!(), line!());
        $( eprint!(" {}", $disp ); )*
        eprintln!();
    };
}

#[cfg(not(feature = "tracing"))]
#[allow(unused_macros)]
macro_rules! fxtrace {
    () => {};
}

#[allow(unused_imports)]
pub(crate) use fxtrace;
pub(crate) use needs_helper;
pub(crate) use set_literals;
pub(crate) use validate_exclusives;
pub(crate) use TODO;
// pub(crate) use self::helper_std_fields;
