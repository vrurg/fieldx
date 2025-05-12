pub mod helper;
pub mod impl_details;
pub mod meta;

#[allow(unused)]
pub enum FXInlining {
    Default,
    Inline,
    Always,
}
