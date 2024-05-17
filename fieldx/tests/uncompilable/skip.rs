use fieldx::fxstruct;

#[fxstruct(lazy, get)]
struct Foo {
    #[fieldx(skip, default(321.654))]
    bare_field: f64,
}

fn main() {
    let foo = Foo::new();
    /*
     * The `get` in struct parameters suggests that `bare_field` must provide a getter. But `skip` tells us otherwise...
     */
    println!("{:?}", foo.bare_field());
}