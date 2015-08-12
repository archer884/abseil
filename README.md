# abseil
Descriptive fallback behavior for Rust result and option types.

Here's some example code (also found in the examples directory in the repo):

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

The *intent* of this library is to provide fallback behavior that takes a pair of concrete types that aren't necessarily the same type and allow them to become a trait object instead, but *that* part hasn't quite worked. Yet.
