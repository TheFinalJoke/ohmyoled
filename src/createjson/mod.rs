mod stock;
mod time;
mod weather;
use oledlib;
struct MatrixOptions {
    chain_length: i8,
    parallel: i8,
    brightness: i32,
    oled_slowdown: i32,
    fail_on_error: bool,
}

pub fn main_menu() {
    println!("Choose From Modules to customize");
    println!("Select a Number");
    println!("1. Time");
    println!("2. Weather");
    println!("3. Stock");
    println!("4. Sports");
    println!("C. Continue");
    println!("Q. Quit");
}

pub fn create_json() {
    let mut modules = vec![];
    let mut main_json = json::object! {};
    loop {
        main_menu();
        match oledlib::get_input() {
            Some(input) => {
                match input.as_str() {
                    "1" => {
                        if modules.contains(&1) {
                            println!("Time is already enabled")
                        } else {
                            modules.push(1);
                        }
                    }
                    "2" => {
                        if modules.contains(&2) {
                            println!("Weather is already enabled")
                        } else {
                            modules.push(2);
                        }
                    }
                    "3" => {
                        if modules.contains(&3) {
                            println!("Stock is already enabled")
                        } else {
                            modules.push(3);
                        }
                    }
                    "4" => {
                        if modules.contains(&4) {
                            println!("Sports are already enabled")
                        } else {
                            modules.push(4);
                        }
                    }
                    "c" => break,
                    "q" => return,
                    _ => break,
                };
            }
            None => return,
        };
    }
    for module in modules {
        match module {
            // will probably have to store or add to the json outright
            1 => {
                let timeoptions = time::configure();
            }
            2 => {
                let weatheroptions = weather::configure();
                dbg!(weatheroptions);
            }
            3 => {
                let stockoptions = stock::configure();
            }
            4 => println!("sport"),
            _ => break,
        }
    }
    dbg!(&main_json["Time"]);
}
