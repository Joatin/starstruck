use starstruck::Starstruck;

fn main() {
    let starstruck = Starstruck::new("01 Simple Window").unwrap();
    starstruck.start_game_loop(|_context| {

    });
}