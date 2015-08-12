# abseil
Convenient fallback behavior for Rust result and option types

I know, you could always do `FallibleType::new().unwrap_or(something())`, but that results in you having to to write a lot of extra code if the two types aren't literally the same type. In contrast, this library allows you to use two types that just share a trait.

Obviously, this only works with object safe traits. Here's some example code (also found in the examples directory in the repo):

    extern crate abseil;
    use abseil::Fallback;

    trait Something {
        fn print(&self);
    }

    impl Something for Option<u32> {
        fn print(&self) { println!("{:?}", self); }
    }

    impl Something for str {
        fn print(&self) { println!("{}", self); }
    }

    fn main() {
        let something = Fallback::from(None).to("Hello");

        something.print();
    }
