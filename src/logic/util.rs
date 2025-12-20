#[macro_export]
macro_rules! unwrap_print {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                println!("Error: {}", e);
                panic!("Unwrap failed");
            }
        }
    };
}
