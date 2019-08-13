pub trait Visitor<T, R> {
    fn visit(&mut self, t: &mut T) -> R;
}

#[macro_export]
macro_rules! impl_visitor {
    (for $name:ident $(< $g:ident : $bound:ident >)? , ($self:ident, $tname:ident : $t:ty) -> $r:ty  $body:block ) => {
        impl $(<$g : $bound>)? Visitor<$t, $r> for $name $(<$g>)? {
            fn visit(&mut $self, $tname: &mut $t) -> $r $body
        }
    };
}
