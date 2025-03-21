use fieldx::fxstruct;

#[fxstruct(get(doc("not here")))]
struct StructLevel {
    value: i32,
}

#[fxstruct()]
struct FieldLevel {
    #[fieldx(builder(post_build))]
    other: i32,
}

#[cfg(feature = "serde")]
#[fxstruct(serde)]
struct FieldLevelSerde {
    #[fieldx(serde(shadow_name("WontWork"), private))]
    value: i32,
}

fn main() {}
