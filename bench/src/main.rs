use jigsaw::comp::Comp;

pub fn main() {
    let state = Comp::example().ser_derived_state();
    if std::env::args().nth(1).is_some() {
        println!("{}", state);
    }
}
