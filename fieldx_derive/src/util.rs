use quote::quote;

pub(crate) mod args;

macro_rules! validate_exclusives {
    ($( $group:expr => $( $field:ident ),+ );+) => {
        fn validate_exclusives(&self) -> ::darling::Result<()> {
            $(
                {
                    // let mut set_params: Vec<(&str, &dyn ::quote::ToTokens)> = vec![];
                    let mut set_params: Vec<&str> = vec![];
                    $(
                        if self.$field.is_some() {
                            // let $field: &dyn ::quote::ToTokens = self.$field.as_ref().unwrap().orig().unwrap();
                            // set_params.push((stringify!($field), $field));
                            set_params.push(stringify!($field));
                        }
                    )+

                    if set_params.len() > 1 {
                        #[allow(unused_mut)]
                        let mut err = darling::Error::custom(
                            format!("The following options from group '{}' cannot be used together: {}", $group, set_params.iter().map(|f| format!("`{}`", f)).collect::<Vec<String>>().join(", "))
                        );

                        #[cfg(feature = "diagnostics")]
                        for field in set_params.iter() {
                            err = err.span_warning(&field.1, format!("`{}` is declared here", field.0));
                        }

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
    ( $name:ident, $min:literal .. $max:literal => $( $field:ident as $ty:path ),+ $( ; pre_validate => $pre_validate:ident )? ) => {
        fn set_literals(#[allow(unused_mut)] mut self, literals: &Vec<Lit>) -> ::darling::Result<Self> {
            $( let _: () = self.$pre_validate(literals)?; )?
            #[allow(unused_comparisons)]
            if literals.len() < $min {
                return Err( darling::Error::custom(format!("Too few literal arguments for {}", stringify!($name))) )
            }
            if literals.len() > $max {
                return Err( darling::Error::custom(format!("Too many literal arguments for {}", stringify!($name))) )
            }
            let mut iter = literals.iter();
            $(
                if let Some(lit) = iter.next() {
                    if let $ty(lit_value) = lit {
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
        fn set_literals(#[allow(unused_mut)] mut self, _literals: &Vec<Lit>) -> ::darling::Result<Self> {
            Err(darling::Error::custom(format!("No literals are allowed with `{}`", stringify!($name))))
        }
    };
}

// macro_rules! helper_std_fields {
//     // Move on to the next syntax struct: generic params, where clauses, or the actual struct body
//     (@ next ( $( $shead:tt )+ ) -> [ { $( $body:tt )* } ] ) => {
//         // This is what is this all about...
//         $( $shead )+ {
//             rename: Option<String>,
//             off: Flag,
//             attributes_fn: Option<FXAttributes>,
//             public: Option<FXNestingAttr<FXPubMode>>,
//             private: Option<FXWithOrig<bool, syn::Meta>>,

//             $( $body )*
//         }
//     };

//     (@ next ( $( $shead:tt )+ ) -> [ < $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gparam ( $( $shead )+ < ) -> [ $( $tail )+ ] }
//     };

//     (@ next ( $( $shead:tt )+ ) -> [ where $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gparam ( $( $shead )+ where ) -> [ $( $tail )+ ] }
//     };

//     // --- Bounds parsing
//     // Lifetime bounds
//     (@ bounds ( $( $shead:tt )+ ) -> [ $lt:lifetime $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ bounds ( $( $shead )+ $lt ) -> [ $( $tail )+ ] }
//     };

//     // Trait name in bounds, possibly an FQN with `::`.
//     (@ bounds ( $( $shead:tt )+ ) -> [ $t:ident $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ bounds ( $( $shead )+ $t ) -> [ $( $tail )+ ] }
//     };
//     (@ bounds ( $( $shead:tt )+ ) -> [ :: $t:ident $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ bounds ( $( $shead )+ :: $t ) -> [ $( $tail )+ ] }
//     };

//     // This one is for `where` which may end with comma followed by struct body
//     (@ bounds ( $( $shead:tt )+ ) -> [ $(,)+ { $( $tail:tt )+ } ] ) => {
//         helper_std_fields!{@ next ( $( $shead )+ , ) -> [ { $( $tail )+ } ] }
//     };

//     (@ bounds ( $( $shead:tt )+ ) -> [ + $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ bounds ( $( $shead )+ + ) -> [ $( $tail )+ ] }
//     };
//     (@ bounds ( $( $shead:tt )+ ) -> [ > $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ next ( $( $shead )+ > ) -> [ $( $tail )+ ] }
//     };
//     (@ bounds ( $( $shead:tt )+ ) -> [ $(,)+ $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gpunct ( $( $shead )+ ) -> [ , $( $tail )+ ] }
//     };
//     (@ bounds ( $( $shead:tt )+ ) -> [ { $( $body:tt )+ } ] ) => {
//         helper_std_fields!{@ next ( $( $shead )+ ) -> [ { $( $body )+ } ] }
//     };

//     // : <bounds>
//     (@ gpunct ( $( $shead:tt )+ ) -> [ : $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ bounds ( $( $shead )+ : ) -> [ $( $tail )+ ] }
//     };

//     // , <next parameter>
//     (@ gpunct ( $( $shead:tt )+ ) -> [ $(,)+ $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gparam ( $( $shead )+ , ) -> [ $( $tail )+ ] }
//     };

//     // > [where | { ... }]
//     (@ gpunct ( $( $shead:tt )+ ) -> [ > $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ next ( $( $shead )+ > ) -> [ $( $tail )+ ] }
//     };

//     // = <literal> (for consts)
//     (@ gpunct ( $( $shead:tt )+ ) -> [ = $lit:literal $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gpunct ( $( $shead )+ = $lit ) -> [ $( $tail )+ ] }
//     };

//     // const param
//     (@ gparam ( $( $shead:tt )+ ) -> [ const $gp:ident : $gtype:ty = $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gpunct ( $( $shead )+ const $gp: $gtype ) -> [ = $( $tail )+ ] }
//     };
//     (@ gparam ( $( $shead:tt )+ ) -> [ const $gp:ident : $gtype:ty , $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gpunct ( $( $shead )+ const $gp: $gtype ) -> [ , $( $tail )+ ] }
//     };

//     // Generic type param
//     (@ gparam ( $( $shead:tt )+ ) -> [ $gname:ident $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gpunct ( $( $shead )+ $gname ) -> [ $( $tail )+ ] }
//     };

//     // Lifetime param
//     (@ gparam ( $( $shead:tt )+ ) -> [ $lf:lifetime $( $tail:tt )+ ] ) => {
//         helper_std_fields!{@ gpunct ( $( $shead )+ $lf ) -> [ $( $tail )+ ] }
//     };

//     ( $( #[$attrs:meta] )* $svis:vis struct $sname:ident $( $tail:tt )* ) => {
//         helper_std_fields!{@ next ( { attrs: [ $(#[$attrs])* ], sname: [ $sname ] } => $svis struct $sname ) -> [ $( $tail )+ ]  }
//     };
// }

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

#[cfg(not(feature = "tracing"))]
pub(crate) use fxtrace;
pub(crate) use needs_helper;
pub(crate) use set_literals;
pub(crate) use validate_exclusives;
// pub(crate) use self::helper_std_fields;
