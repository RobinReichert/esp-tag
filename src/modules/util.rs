
#[macro_export]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

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
