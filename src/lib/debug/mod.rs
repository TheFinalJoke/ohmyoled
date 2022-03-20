use std::any::type_name;

// Sometimes you need to know the type of a variable. how you call below
// use oledlib::debug;
// println!("{}", debug::type_of(self.api_key.clone().unwrap().as_str()));
pub fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}
