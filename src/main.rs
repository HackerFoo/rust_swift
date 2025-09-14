#[swift_bridge::bridge]
pub mod ffi {
    extern "Swift" {
        fn getMessage() -> String;
    }
}

fn main() {
    println!("{}", ffi::getMessage())
}
