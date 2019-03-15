

pub trait ComponentBuilder<T: Component> {
    fn build() -> Box<T>;
}