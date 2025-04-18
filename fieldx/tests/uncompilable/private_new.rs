mod foo {
    use fieldx::fxstruct;

    #[fxstruct(new(private))]
    pub struct Foo {
        #[fieldx(get(clone), set, into)]
        bar: String,
    }

    #[fxstruct(new("_new", private))]
    pub struct Bar {
        #[fieldx(get(clone), set, into)]
        bar: String,
    }
}

fn main() {
    let foo = foo::Foo::__fieldx_new();
    let bar = foo::Bar::_new();
}
