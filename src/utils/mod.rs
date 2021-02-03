use std::env;
use clap::ArgMatches;
use std::process::exit;

pub fn get_working_dir() -> String {
    match env::current_dir() {
        Ok(current_dir) => current_dir.display().to_string(),
        _ => {
            println!("Something went wrong!");
            exit(1)
        }
    }
}

pub fn get_variable(name: &str, args: Option<&ArgMatches>, default: String) -> Option<String> {
    let mut variable_value: Option<String> = None;
    match env::var(name) {
        Ok(val) => variable_value = Option::from(val),
        Err(_e) => {
            if let Some(existing_args) = args {
                match existing_args.value_of(name) {
                    Some(val) => {
                        variable_value = Option::from(val.to_string());
                    }
                    None => variable_value = Option::from(default)
                }
            }
        },
    }
    variable_value
}
