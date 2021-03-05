use proj::comp::Comp;

pub fn main() {
    let state = Comp::example().derived_state();
    if std::env::args().nth(1).is_some() {
        println!("{}", state);
    }
}
