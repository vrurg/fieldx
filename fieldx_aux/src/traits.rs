pub trait FXTriggerHelper {
    fn is_true(&self) -> bool;
}

pub trait FXFrom<T> {
    fn fx_from(value: T) -> Self;
}

pub trait FXInto<T> {
    fn fx_into(self) -> T;
}

pub trait FXBoolHelper {
    fn is_true(&self) -> bool;
    fn is_true_opt(&self) -> Option<bool>;
    fn not_true(&self) -> bool {
        !self.is_true()
    }
    // fn not_true_opt(&self) -> Option<bool> {
    //     self.is_true_opt().map(|b| !b)
    // }
}

impl<T, U> FXInto<U> for T
where
    U: FXFrom<T>,
{
    #[inline]
    fn fx_into(self) -> U {
        U::fx_from(self)
    }
}

impl<H: FXTriggerHelper> FXBoolHelper for Option<H> {
    #[inline]
    fn is_true(&self) -> bool {
        self.as_ref().map_or(false, |h| h.is_true())
    }

    #[inline]
    fn is_true_opt(&self) -> Option<bool> {
        self.as_ref().map(|h| h.is_true())
    }
}
