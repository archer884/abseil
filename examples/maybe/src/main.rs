extern crate abseil;
use abseil::Fallback;

trait Something {
    fn print(&self);
}

impl Something for str {
    fn print(&self) { println!("{}", self); }
}

fn main() {
    let something = Fallback::from(None).to("Hello");

    something.print();
}
