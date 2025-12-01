// #[macro_export]
// macro_rules! env_parse_internal {
//     ($name:expr, $typ:ty) => {{
//         const fn parse_str_to_num(s: &str) -> $typ {
//             let mut result: $typ = 0;
//             let bytes = s.as_bytes();
//             let mut i = 0;

//             while i < bytes.len() {
//                 let digit = bytes[i];
//                 if digit < b'0' || digit > b'9' {
//                     panic!("Invalid digit in environment variable");
//                 }
//                 result = result * 10 + (digit - b'0') as $typ;
//                 i += 1;
//             }
//             result
//         }
//         parse_str_to_num(env!($name))
//     }};
// }

// #[macro_export]
// macro_rules! env_u64 {
//     ($name:expr) => {
//         $crate::env_parse_internal!($name, u64)
//     };
// }

// #[macro_export]
// macro_rules! env_u32 {
//     ($name:expr) => {
//         $crate::env_parse_internal!($name, u32)
//     };
// }

// #[macro_export]
// macro_rules! env_bool {
//     ($name:expr) => {{
//         const fn parse_str_to_bool(s: &str) -> bool {
//             let bytes = s.as_bytes();
//             if bytes.len() == 4
//                 && (bytes[0] == b't' || bytes[0] == b'T')
//                 && (bytes[1] == b'r' || bytes[1] == b'R')
//                 && (bytes[2] == b'u' || bytes[2] == b'U')
//                 && (bytes[3] == b'e' || bytes[3] == b'E')
//             {
//                 return true;
//             }
//             false
//         }
//         parse_str_to_bool(env!($name))
//     }};
// }

// #[macro_export]
// macro_rules! env_str {
//     ($name:expr) => {{ env!($name) }};
// }

#[cfg(test)]
use log::LevelFilter;

#[cfg(test)]
pub fn init_logging(level: LevelFilter) {
    let _ = env_logger::builder()
        .filter_level(level)
        .is_test(true)
        .try_init();
}
