/// Generate implementation of `set_literals` method for [`FromNestAttr`](crate::nesting_attr::FromNestAttr) trait.
#[macro_export]
macro_rules! set_literals {
    ( $name:ident, $min:literal .. $($max:literal)? => $( $field:ident ),+ $( ; pre_validate => $pre_validate:ident )? ) => {
        fn set_literals(#[allow(unused_mut)] mut self, literals: &[syn::Lit]) -> ::darling::Result<Self> {
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
                    self.$field =
                        (stringify!($field), lit.clone()).fx_try_into().map_err(|e| darling::Error::from(e).with_span(lit))?;
                }
            )+
            Ok(self)
        }
    };
    ($name:ident, .. $max:tt => $( $field:ident ),+ $( ; pre_validate => $pre_validate:ident )? ) => {
        set_literals! {$name, 0 .. $max => $( $field ),+ $( ; pre_validate => $pre_validate )?}
    };
    ($name:ident $(, .. 0 )?) => {
        fn set_literals(#[allow(unused_mut)] mut self, _literals: &Vec<::syn::Lit>) -> ::darling::Result<Self> {
            Err(darling::Error::custom(format!("No literals are allowed with `{}`", stringify!($name))))
        }
    };
}

#[macro_export]
macro_rules! ident_or_alias {
    ($name:ident) => {
        stringify!($name)
    };
    ($name:ident, $alias:ident) => {
        stringify!($alias)
    };
    ($name:ident, $alias:literal) => {
        $alias
    };
}

/// Generate `validate_exclusives` that would return a [`darling::Result`] if two arguments of an attribute are
/// conflicting with each other.
#[macro_export]
macro_rules! validate_exclusives {
    (or_alias: $name:expr, ) => {
        $name
    };
    (or_alias: $name:expr, $or_as:literal) => {
        $or_as
    };

    ( $( $group:literal: $( $( $field:ident $( as $alias:literal )? ),+ );+ ; )+ ) => {
        fn validate_exclusives(&self) -> ::darling::Result<()> {
            // Though use of a HashMap is tempting but vectors allow to preserve the order declarations.
            use ::quote::ToTokens;
            let mut groups = vec![];
            $(
                groups.push( ($group, vec![]) );
                let exclusives = &mut groups.last_mut().unwrap().1;
                $(
                    exclusives.push(vec![]);
                    let subgroup = exclusives.last_mut().unwrap();
                    $(
                        let field_ref = self.$field.as_ref();
                        subgroup.push( ( $crate::ident_or_alias!($field $(, $alias )? ),
                                            field_ref.map(|f| *f.is_set()).unwrap_or(false),
                                            field_ref.map(|f| f.final_span()) ) );
                    )+
                )+
            )+

            let mut all_errs = vec![];

            for (group, exclusives) in groups {
                let mut set_params = vec![];
                let mut subgroups_set = 0;

                for subgroup in exclusives {
                    let mut subgroup_set = false;
                    for (name, is_set, span) in subgroup {
                        if is_set {
                            subgroup_set = true;
                            set_params.push((name, span));
                        }
                    }

                    if subgroup_set {
                        subgroups_set += 1;
                    }
                }


                if subgroups_set > 1 {
                    let mut errs = vec![];
                    errs.push(
                        darling::Error::custom(
                            format!(
                                "Conflicting arguments {} from group '{}' cannot be used at the same time",
                                set_params
                                    .iter()
                                    .map(|f| format!("'{}'", f.0))
                                    .collect::<Vec<String>>()
                                    .join(", "),
                                group,
                            )
                        )
                    );

                    for (name, span) in set_params {
                            let err = darling::Error::custom(format!("Argument '{}' is located here", name));
                            if let Some(span) = span {
                                errs.push(err.with_span(&span));
                            }
                            else {
                                errs.push(err);
                            }
                    }

                    all_errs.push(darling::Error::multiple(errs));
                }
            }

            if all_errs.len() > 0 {
                Err(darling::Error::multiple(all_errs))
            }
            else {
                Ok(())
            }
        }
    };
}

/// Generate function to produce errors on sub-arguments that won't be used in certain context.
#[macro_export]
macro_rules! validate_no_macro_args {
    ($level:literal, $self:ident , $accumulator:ident :
        $(
            $arg:ident $( as $alias:ident )? . $subarg:ident $( as $sub_alias:ident )?
        ),+
        $(,)?
    ) => {
            $(
                if let Some(ref arg) = $self.$arg() {
                    let is_set = arg.$subarg().is_set();
                    if *is_set {
                        $accumulator.push(
                            darling::Error::custom(
                                format!(
                                    "'{}' has no effect in '{}' context at {} level",
                                    $crate::ident_or_alias!($subarg $(, $sub_alias )?),
                                    $crate::ident_or_alias!($arg $(, $alias)?),
                                    $level,
                                )
                            )
                            .with_span(&arg.$subarg().final_span())
                        );
                    }
                }
            )+
    };
}
