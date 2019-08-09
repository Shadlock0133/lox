pub trait Visitor<T, R> {
    fn visit(&mut self, t: &mut T) -> R;
}
