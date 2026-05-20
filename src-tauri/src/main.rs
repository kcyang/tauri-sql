// Windows 릴리즈에서 추가 콘솔 윈도우 방지
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    just_sql_lib::run()
}
