use uart_handler::command::Command;

use heapless::String;

#[test]
fn test_conversion_to_str() {
    let cmd = Command::Station;

    let cmd_str: String<3> = cmd.to_string();

    // let mut s = String::<3>::new();
    // s.push_str("STA");
    assert_eq!("STA", cmd_str);
}
